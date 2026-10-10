//! Server deployment orchestrator (`grs deploy`).

use crate::config::{BackendType, FilePatchRule, LocalConfig};
use crate::patch::{apply_key_value_patch, apply_list_entry_patch, evaluate_template};
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
        .or_else(|| config.deploy.server_path.clone())
        .ok_or_else(|| {
            "Server directory not specified. Provide --path or set deploy.server_path in .goldsrc.toml"
                .to_string()
        })?;

    if !server_root.exists() {
        return Err(format!(
            "Target server path does not exist: {}",
            server_root.display()
        ));
    }

    let mod_name = &config.deploy.mod_name;
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

    let backend = opts.backend.unwrap_or(config.deploy.backend);
    println!("Configured backend: {:?}", backend);

    // Auto-cache settings back to .goldsrc.toml if flags were passed or changed
    let should_save_cache = (opts.server_path.is_some()
        && config.deploy.server_path.as_deref() != Some(&server_root))
        || (opts.backend.is_some() && config.deploy.backend != backend);

    if should_save_cache {
        let mut updated_config = config.clone();
        updated_config.deploy.server_path = Some(server_root.clone());
        updated_config.deploy.backend = backend;
        let config_path =
            LocalConfig::find_config_path().unwrap_or_else(|| PathBuf::from(".goldsrc.toml"));
        if let Err(e) = updated_config.save_deploy_cache(&config_path) {
            eprintln!(
                "Warning: Failed to update deployment cache in {}: {e}",
                config_path.display()
            );
        } else {
            println!("Cached deployment settings in {}", config_path.display());
        }
    }

    // Resolve environment and target variables
    let host_triple = if cfg!(windows) {
        "i686-pc-windows-msvc"
    } else if cfg!(target_os = "macos") {
        "x86_64-apple-darwin"
    } else {
        "i686-unknown-linux-gnu"
    };

    let vars = config.resolve_variables(host_triple);

    // Execute declarative file patches for selected backend
    let backend_def = config.get_backend_definition(backend);

    for patch in &backend_def.patches {
        match patch {
            FilePatchRule::ListEntry {
                file,
                comment_prefixes,
                match_pattern,
                entry,
            } => {
                let resolved_file_rel = evaluate_template(file, &vars);
                let target_file = target_mod_dir.join(&resolved_file_rel);
                let resolved_entry = evaluate_template(entry, &vars);
                let resolved_match = evaluate_template(match_pattern, &vars);

                println!("Patching list entry in: {}", target_file.display());
                apply_list_entry_patch(
                    &target_file,
                    &resolved_entry,
                    &resolved_match,
                    comment_prefixes,
                )?;
            }
            FilePatchRule::KeyValue {
                file,
                separator,
                key,
                value,
            } => {
                let resolved_file_rel = evaluate_template(file, &vars);
                let target_file = target_mod_dir.join(&resolved_file_rel);
                let resolved_key = evaluate_template(key, &vars);
                let resolved_value = evaluate_template(value, &vars);

                println!(
                    "Patching key-value '{}' in: {}",
                    resolved_key,
                    target_file.display()
                );
                apply_key_value_patch(&target_file, &resolved_key, &resolved_value, separator)?;
            }
        }
    }

    // Deploy binary libraries if copy_dest is defined
    if !backend_def.copy_dest.is_empty() {
        let resolved_copy_dest = evaluate_template(&backend_def.copy_dest, &vars);
        let dest_dir = target_mod_dir.join(&resolved_copy_dest);
        fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;

        // Locate compiled runtime binary
        let lib_prefix = vars.get("lib_prefix").map(|s| s.as_str()).unwrap_or("");
        let lib_ext = vars.get("lib_ext").map(|s| s.as_str()).unwrap_or("dll");
        let bin_name = match backend {
            BackendType::Metamod => format!("{lib_prefix}goldsrc_metamod.{lib_ext}"),
            BackendType::Standalone => format!("{lib_prefix}goldsrc_standalone.{lib_ext}"),
        };

        // Check common target build locations (prioritizing host_triple specific builds)
        let candidate_sources = [
            PathBuf::from(format!(
                "../goldsrc-runtime/target/{host_triple}/release/{bin_name}"
            )),
            PathBuf::from(format!("target/{host_triple}/release/{bin_name}")),
            PathBuf::from(format!(
                "../goldsrc-runtime/target/{host_triple}/debug/{bin_name}"
            )),
            PathBuf::from(format!("target/{host_triple}/debug/{bin_name}")),
            PathBuf::from(format!("../goldsrc-runtime/target/release/{bin_name}")),
            PathBuf::from(format!("target/release/{bin_name}")),
        ];

        for src in &candidate_sources {
            if src.exists() {
                let dest = dest_dir.join(&bin_name);
                println!(
                    "Deploying runtime binary: {} -> {}",
                    src.display(),
                    dest.display()
                );
                fs::copy(src, &dest).map_err(|e| e.to_string())?;
                break;
            }
        }
    }

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
