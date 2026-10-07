//! Local environment and server configuration loader (`.local.toml`).

use serde::{Deserialize, Serialize};
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

impl BackendType {
    /// Destination library filename for the backend.
    #[allow(dead_code)]
    pub fn lib_filename(&self, is_windows: bool) -> String {
        let base = match self {
            Self::Metamod => "goldsrc_metamod",
            Self::Standalone => "goldsrc_standalone",
        };
        if is_windows {
            format!("{base}.dll")
        } else {
            format!("lib{base}.so")
        }
    }
}

/// Server deployment and test settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Path to the root HLDS / ReHLDS directory.
    pub hlds_dir: Option<PathBuf>,
    /// Game mod directory (default: "cstrike").
    #[serde(default = "default_mod")]
    pub mod_name: String,
    /// Default backend to deploy.
    #[serde(default)]
    pub backend: BackendType,
}

fn default_mod() -> String {
    "cstrike".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            hlds_dir: None,
            mod_name: default_mod(),
            backend: BackendType::default(),
        }
    }
}

/// References and codegen paths.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodegenConfig {
    /// Path to offline C/C++ engine references repository.
    pub references_dir: Option<PathBuf>,
}

/// Root `.local.toml` configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalConfig {
    /// Server environment configuration.
    #[serde(default)]
    pub server: ServerConfig,
    /// Codegen and headers paths.
    #[serde(default)]
    pub codegen: CodegenConfig,
}

impl LocalConfig {
    /// Locate and parse `.local.toml` starting from the current directory upwards.
    pub fn load() -> Self {
        if let Ok(current_dir) = std::env::current_dir() {
            let mut curr: &Path = &current_dir;
            loop {
                let candidate = curr.join(".local.toml");
                if candidate.is_file() {
                    let parsed = std::fs::read_to_string(&candidate)
                        .ok()
                        .and_then(|c| toml::from_str::<LocalConfig>(&c).ok());
                    if let Some(cfg) = parsed {
                        return cfg;
                    }
                }
                match curr.parent() {
                    Some(parent) => curr = parent,
                    None => break,
                }
            }
        }
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_local_config() {
        let toml_str = r#"
            [server]
            hlds_dir = "C:/hlds"
            mod_name = "cstrike"
            backend = "standalone"

            [codegen]
            references_dir = "D:/Repo/GoldSrc.rs/references"
        "#;
        let config: LocalConfig = toml::from_str(toml_str).expect("Valid config");
        assert_eq!(config.server.mod_name, "cstrike");
        assert_eq!(config.server.backend, BackendType::Standalone);
        assert_eq!(config.server.hlds_dir, Some(PathBuf::from("C:/hlds")));
        assert_eq!(
            config.codegen.references_dir,
            Some(PathBuf::from("D:/Repo/GoldSrc.rs/references"))
        );
    }
}
