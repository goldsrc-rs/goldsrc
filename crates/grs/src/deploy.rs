//! Server deployment orchestrator (`grs deploy`).

use crate::config::{BackendType, LocalConfig};
use std::fs;
use std::path::{Path, PathBuf};

/// Options for deploying plugins or runtime to a server.
pub struct DeployOptions<'a> {
    pub server_path: Option<&'a Path>,
    pub backend: Option<BackendType>,
    pub verify_only: bool,
}

pub fn execute_deploy(opts: DeployOptions<'_>, config: &LocalConfig) -> Result<(), String> {
    let server_root = opts
        .server_path
        .map(PathBuf::from)
        .or_else(|| config.server.hlds_dir.clone())
        .ok_or_else(|| {
            "Server directory not specified. Provide --path or set server.hlds_dir in .local.toml"
                .to_string()
        })?;

    if !server_root.exists() {
        return Err(format!(
            "Target server path does not exist: {}",
            server_root.display()
        ));
    }

    let mod_name = &config.server.mod_name;
    let target_mod_dir = server_root.join(mod_name);
    if !target_mod_dir.exists() {
        return Err(format!(
            "Game mod directory '{}' not found inside server root: {}",
            mod_name,
            target_mod_dir.display()
        ));
    }

    let goldsrc_dir = target_mod_dir.join("addons").join("goldsrc");
    let plugins_dir = goldsrc_dir.join("plugins");
    let lang_dir = goldsrc_dir.join("lang");

    println!("Deploy target: {}", goldsrc_dir.display());

    if opts.verify_only {
        println!("Verifying deployment structure...");
        if !goldsrc_dir.exists() {
            return Err("addons/goldsrc directory not found".to_string());
        }
        println!("Verification OK: Deployment directory exists.");
        return Ok(());
    }

    fs::create_dir_all(&plugins_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&lang_dir).map_err(|e| e.to_string())?;

    let backend = opts.backend.unwrap_or(config.server.backend);
    println!("Configured backend: {:?}", backend);

    // Scan for compiled .wasm plugins in target/wasm32-unknown-unknown/release
    let wasm_source_dir = Path::new("target/wasm32-unknown-unknown/release");
    if let Ok(entries) = fs::read_dir(wasm_source_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                let dest = plugins_dir.join(path.file_name().unwrap());
                println!("Deploying plugin: {} -> {}", path.display(), dest.display());
                fs::copy(&path, &dest).map_err(|e| e.to_string())?;
            }
        }
    }

    // Deploy localization dictionaries if present in resources/lang
    let lang_source_dir = Path::new("resources/lang");
    if let Ok(entries) = fs::read_dir(lang_source_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                let dest = lang_dir.join(path.file_name().unwrap());
                println!(
                    "Deploying dictionary: {} -> {}",
                    path.display(),
                    dest.display()
                );
                fs::copy(&path, &dest).map_err(|e| e.to_string())?;
            }
        }
    }

    println!("Deployment completed successfully.");
    Ok(())
}
