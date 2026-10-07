//! Project scaffolding command (`grs new`).

use std::fs;
use std::path::PathBuf;

/// Options for scaffolding a new plugin.
pub struct NewOptions<'a> {
    pub name: &'a str,
    pub game: Option<&'a str>,
    #[allow(dead_code)]
    pub template: &'a str,
}

pub fn scaffold_plugin(opts: NewOptions<'_>) -> Result<PathBuf, String> {
    let target_dir = PathBuf::from(opts.name);
    if target_dir.exists() {
        return Err(format!("Directory '{}' already exists", opts.name));
    }

    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
    let src_dir = target_dir.join("src");
    fs::create_dir_all(&src_dir).map_err(|e| e.to_string())?;

    let is_cstrike = opts.game == Some("cstrike");
    let name = opts.name;

    // Generate Cargo.toml
    let cargo_toml = if is_cstrike {
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"
authors = ["GoldSrc.rs Plugin Author"]
description = "Counter-Strike 1.6 WebAssembly plugin"

[lib]
crate-type = ["cdylib"]

[dependencies]
cstrike-sdk = "0.20"
"#,
            name = name
        )
    } else {
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"
authors = ["GoldSrc.rs Plugin Author"]
description = "GoldSrc WebAssembly plugin"

[lib]
crate-type = ["cdylib"]

[dependencies]
goldsrc-sdk = "0.20"
"#,
            name = name
        )
    };
    fs::write(target_dir.join("Cargo.toml"), cargo_toml).map_err(|e| e.to_string())?;

    // Generate src/lib.rs based on game
    let lib_rs = if is_cstrike {
        r#"//! Counter-Strike 1.6 WebAssembly Plugin.

use cstrike_sdk::prelude::*;

#[plugin]
pub struct CsPlugin;

impl Plugin for CsPlugin {
    fn on_load(&mut self) -> Result<()> {
        log_info!("CS 1.6 Plugin initialized successfully!");
        Ok(())
    }
}
"#
    } else {
        r#"//! GoldSrc WebAssembly Plugin.

use goldsrc_sdk::prelude::*;

#[plugin]
pub struct EnginePlugin;

impl Plugin for EnginePlugin {
    fn on_load(&mut self) -> Result<()> {
        log_info!("GoldSrc Plugin initialized successfully!");
        Ok(())
    }
}
"#
    };
    fs::write(src_dir.join("lib.rs"), lib_rs).map_err(|e| e.to_string())?;

    // Generate .gitignore
    let gitignore = "/target\nCargo.lock\n";
    fs::write(target_dir.join(".gitignore"), gitignore).map_err(|e| e.to_string())?;

    Ok(target_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaffold_default_engine_plugin() {
        let temp = std::env::temp_dir().join(format!("test_grs_new_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let opts = NewOptions {
            name: temp.to_str().unwrap(),
            game: None,
            template: "minimal",
        };
        let res = scaffold_plugin(opts);
        assert!(res.is_ok());

        let cargo_toml = fs::read_to_string(temp.join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains("goldsrc-sdk"));
        assert!(!cargo_toml.contains("cstrike-sdk"));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_scaffold_cstrike_plugin() {
        let temp = std::env::temp_dir().join(format!("test_grs_new_cs_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let opts = NewOptions {
            name: temp.to_str().unwrap(),
            game: Some("cstrike"),
            template: "minimal",
        };
        let res = scaffold_plugin(opts);
        assert!(res.is_ok());

        let cargo_toml = fs::read_to_string(temp.join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains("cstrike-sdk"));

        let _ = fs::remove_dir_all(&temp);
    }
}
