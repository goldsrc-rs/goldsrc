//! Local environment, server deployment, and target-conditional configuration loader (`.goldsrc.toml` / `.local.toml`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Target game engine backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    /// Metamod-r plugin (.dll / .so)
    #[default]
    Metamod,
    /// Standalone GameDLL proxy replacement
    Standalone,
}

impl std::str::FromStr for BackendType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "metamod" => Ok(Self::Metamod),
            "standalone" => Ok(Self::Standalone),
            other => Err(format!("Unknown backend: {other}")),
        }
    }
}

/// A declarative file patch rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FilePatchRule {
    /// List entry in a line-based file (e.g. `plugins.ini`).
    #[serde(rename = "list_entry")]
    ListEntry {
        file: String,
        #[serde(default)]
        comment_prefixes: Vec<String>,
        #[serde(default)]
        match_pattern: String,
        entry: String,
    },
    /// Key-value assignment in a structured config (e.g. `liblist.gam`).
    #[serde(rename = "key_value")]
    KeyValue {
        file: String,
        #[serde(default = "default_separator")]
        separator: String,
        key: String,
        value: String,
    },
}

fn default_separator() -> String {
    " ".to_string()
}

/// Custom backend deployment definition.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackendDeployDef {
    /// Destination directory for copying binaries (relative to game mod directory or `..`).
    #[serde(default)]
    pub copy_dest: String,
    /// Ordered file patches to apply for this backend.
    #[serde(default)]
    pub patches: Vec<FilePatchRule>,
}

/// Target-specific section with variables (e.g. `[target.'cfg(windows)'.variables]`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TargetSection {
    #[serde(default)]
    pub variables: HashMap<String, String>,
}

/// Server deployment settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployConfig {
    /// Path to root HLDS / ReHLDS directory.
    pub server_path: Option<PathBuf>,
    /// Game mod directory (default: "cstrike").
    #[serde(default = "default_mod")]
    pub mod_name: String,
    /// Default backend to deploy.
    #[serde(default)]
    pub backend: BackendType,
    /// Shared template variables for all platforms.
    #[serde(default)]
    pub variables: HashMap<String, String>,
    /// Custom backend deployment rules (keyed by backend name).
    #[serde(default)]
    pub backend_defs: HashMap<String, BackendDeployDef>,
}

fn default_mod() -> String {
    "cstrike".to_string()
}

impl Default for DeployConfig {
    fn default() -> Self {
        Self {
            server_path: None,
            mod_name: default_mod(),
            backend: BackendType::default(),
            variables: HashMap::new(),
            backend_defs: HashMap::new(),
        }
    }
}

/// References and codegen paths.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodegenConfig {
    /// Path to offline C/C++ engine references repository.
    pub references_dir: Option<PathBuf>,
}

/// Root `.goldsrc.toml` configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalConfig {
    /// Deployment configuration.
    #[serde(default)]
    pub deploy: DeployConfig,
    /// Target cfg conditions: `target.'cfg(...)'.variables`.
    #[serde(default)]
    pub target: HashMap<String, TargetSection>,
    /// Codegen and headers paths.
    #[serde(default)]
    pub codegen: CodegenConfig,
}

impl LocalConfig {
    /// Returns the active config path if found, or standard candidate path.
    pub fn find_config_path() -> Option<PathBuf> {
        if let Ok(current_dir) = std::env::current_dir() {
            let mut curr: &Path = &current_dir;
            loop {
                let candidate1 = curr.join(".goldsrc.toml");
                if candidate1.is_file() {
                    return Some(candidate1);
                }
                let candidate2 = curr.join("goldsrc.toml");
                if candidate2.is_file() {
                    return Some(candidate2);
                }
                let candidate3 = curr.join(".local.toml");
                if candidate3.is_file() {
                    return Some(candidate3);
                }
                match curr.parent() {
                    Some(parent) => curr = parent,
                    None => break,
                }
            }
        }
        None
    }

    /// Locate and parse `.goldsrc.toml` (or fallback to `.local.toml`) starting from current dir upwards.
    pub fn load() -> Self {
        if let Some(c) = Self::find_config_path().and_then(|p| std::fs::read_to_string(p).ok()) {
            // First try full LocalConfig format with [deploy]
            if let Ok(cfg) = toml::from_str::<LocalConfig>(&c) {
                return cfg;
            }
            // Also support legacy `.local.toml` with [server]
            #[derive(Deserialize)]
            struct LegacyServer {
                hlds_dir: Option<PathBuf>,
                mod_name: Option<String>,
                backend: Option<BackendType>,
            }
            #[derive(Deserialize)]
            struct LegacyConfig {
                server: Option<LegacyServer>,
                codegen: Option<CodegenConfig>,
            }
            if let Ok(legacy) = toml::from_str::<LegacyConfig>(&c) {
                let mut cfg = LocalConfig::default();
                if let Some(s) = legacy.server {
                    cfg.deploy.server_path = s.hlds_dir;
                    if let Some(m) = s.mod_name {
                        cfg.deploy.mod_name = m;
                    }
                    if let Some(b) = s.backend {
                        cfg.deploy.backend = b;
                    }
                }
                if let Some(cg) = legacy.codegen {
                    cfg.codegen = cg;
                }
                return cfg;
            }
        }
        Self::default()
    }

    /// Saves updated deployment settings back into `.goldsrc.toml`.
    pub fn save_deploy_cache(&self, path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, content).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Evaluates target `cfg(...)` conditions and builds the merged variables map.
    pub fn resolve_variables(&self, target_triple: &str) -> HashMap<String, String> {
        let is_windows = target_triple.contains("windows");
        let is_linux = target_triple.contains("linux");
        let is_macos = target_triple.contains("darwin") || target_triple.contains("apple");

        let mut vars = HashMap::new();

        // 1. Built-in base platform variables
        if is_windows {
            vars.insert("platform".to_string(), "windows".to_string());
            vars.insert("metamod_platform".to_string(), "win32".to_string());
            vars.insert("liblist_key".to_string(), "gamedll".to_string());
            vars.insert("lib_prefix".to_string(), "".to_string());
            vars.insert("lib_ext".to_string(), "dll".to_string());
            vars.insert("path_sep".to_string(), "\\".to_string());
        } else if is_linux {
            vars.insert("platform".to_string(), "linux".to_string());
            vars.insert("metamod_platform".to_string(), "linux".to_string());
            vars.insert("liblist_key".to_string(), "gamedll_linux".to_string());
            vars.insert("lib_prefix".to_string(), "lib".to_string());
            vars.insert("lib_ext".to_string(), "so".to_string());
            vars.insert("path_sep".to_string(), "/".to_string());
        } else if is_macos {
            vars.insert("platform".to_string(), "macos".to_string());
            vars.insert("metamod_platform".to_string(), "osx".to_string());
            vars.insert("liblist_key".to_string(), "gamedll_osx".to_string());
            vars.insert("lib_prefix".to_string(), "lib".to_string());
            vars.insert("lib_ext".to_string(), "dylib".to_string());
            vars.insert("path_sep".to_string(), "/".to_string());
        } else {
            vars.insert("platform".to_string(), "unknown".to_string());
            vars.insert("metamod_platform".to_string(), "".to_string());
            vars.insert("liblist_key".to_string(), "gamedll".to_string());
            vars.insert("lib_prefix".to_string(), "".to_string());
            vars.insert("lib_ext".to_string(), "so".to_string());
            vars.insert("path_sep".to_string(), "/".to_string());
        }

        vars.insert("goldsrc_root".to_string(), "goldsrc".to_string());
        vars.insert("mod_name".to_string(), self.deploy.mod_name.clone());

        // 2. Cascade shared `[deploy.variables]`
        for (k, v) in &self.deploy.variables {
            vars.insert(k.clone(), v.clone());
        }

        // 3. Evaluate conditional `[target.'cfg(...)'.variables]`
        for (cfg_cond, section) in &self.target {
            if evaluate_cfg_condition(cfg_cond, is_windows, is_linux, is_macos) {
                for (k, v) in &section.variables {
                    vars.insert(k.clone(), v.clone());
                }
            }
        }

        vars
    }

    /// Returns built-in fallback patches for Metamod and Standalone if not customized.
    pub fn get_backend_definition(&self, backend: BackendType) -> BackendDeployDef {
        let name = match backend {
            BackendType::Metamod => "metamod",
            BackendType::Standalone => "standalone",
        };

        if let Some(custom) = self.deploy.backend_defs.get(name) {
            return custom.clone();
        }

        match backend {
            BackendType::Metamod => BackendDeployDef {
                copy_dest: "addons/goldsrc/lib".to_string(),
                patches: vec![FilePatchRule::ListEntry {
                    file: "addons/metamod/plugins.ini".to_string(),
                    comment_prefixes: vec![";".to_string(), "//".to_string()],
                    match_pattern: "goldsrc_metamod".to_string(),
                    entry: "{metamod_platform} addons/goldsrc/lib/{lib_prefix}goldsrc_metamod.{lib_ext}".to_string(),
                }],
            },
            BackendType::Standalone => BackendDeployDef {
                copy_dest: "../goldsrc".to_string(),
                patches: vec![FilePatchRule::KeyValue {
                    file: "liblist.gam".to_string(),
                    separator: " ".to_string(),
                    key: "{liblist_key}".to_string(),
                    value: "\"..{path_sep}goldsrc{path_sep}{lib_prefix}goldsrc_standalone.{lib_ext}\"".to_string(),
                }],
            },
        }
    }
}

/// Evaluates standard Cargo `cfg(...)` expressions against target environment flags.
fn evaluate_cfg_condition(cond: &str, is_windows: bool, is_linux: bool, is_macos: bool) -> bool {
    let clean = cond.trim();
    if clean == "cfg(windows)" || clean == "windows" {
        return is_windows;
    }
    if clean == "cfg(unix)" || clean == "unix" {
        return is_linux || is_macos;
    }
    if clean.contains("target_os = \"linux\"") || clean == "linux" {
        return is_linux;
    }
    if clean.contains("target_os = \"windows\"") {
        return is_windows;
    }
    if clean.contains("target_os = \"macos\"") || clean == "macos" {
        return is_macos;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_variables_windows() {
        let toml_str = r#"
            [deploy]
            server_path = "C:/hlds"
            backend = "metamod"

            [deploy.variables]
            custom_brand = "my_server"

            [target.'cfg(windows)'.variables]
            custom_tag = "win32_bin"

            [target.'cfg(unix)'.variables]
            custom_tag = "unix_bin"
        "#;

        let config: LocalConfig = toml::from_str(toml_str).unwrap();
        let vars = config.resolve_variables("i686-pc-windows-msvc");

        assert_eq!(vars.get("platform").unwrap(), "windows");
        assert_eq!(vars.get("lib_ext").unwrap(), "dll");
        assert_eq!(vars.get("custom_brand").unwrap(), "my_server");
        assert_eq!(vars.get("custom_tag").unwrap(), "win32_bin");
    }

    #[test]
    fn test_resolve_variables_linux() {
        let toml_str = r#"
            [deploy]
            server_path = "/opt/hlds"
            backend = "standalone"

            [target.'cfg(unix)'.variables]
            custom_tag = "unix_bin"
        "#;

        let config: LocalConfig = toml::from_str(toml_str).unwrap();
        let vars = config.resolve_variables("i686-unknown-linux-gnu");

        assert_eq!(vars.get("platform").unwrap(), "linux");
        assert_eq!(vars.get("lib_prefix").unwrap(), "lib");
        assert_eq!(vars.get("lib_ext").unwrap(), "so");
        assert_eq!(vars.get("custom_tag").unwrap(), "unix_bin");
    }
}
