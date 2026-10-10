use crate::config::{BackendType, LocalConfig};
use std::path::PathBuf;
use std::process::Command;

/// Options for building GoldSrc plugins or backends.
pub struct BuildOptions<'a> {
    pub backend: Option<BackendType>,
    pub package: Option<&'a str>,
    pub target: Option<&'a str>,
    pub workspace: bool,
    pub release: Option<bool>,
    pub preset: Option<&'a str>,
    pub debug_level: Option<&'a str>,
    pub strip: Option<&'a str>,
}

pub fn execute_build(opts: BuildOptions<'_>, config: &LocalConfig) -> Result<(), String> {
    // 1. Determine effective backend (from CLI flag or cached build.backend)
    let effective_backend = opts.backend.or(config.build.backend);

    // 2. Resolve default target architecture
    let default_target = if effective_backend.is_some() {
        if cfg!(windows) {
            "i686-pc-windows-msvc"
        } else if cfg!(target_os = "macos") {
            "x86_64-apple-darwin"
        } else {
            "i686-unknown-linux-gnu"
        }
    } else {
        "wasm32-unknown-unknown"
    };

    let target = opts
        .target
        .or(config.build.target.as_deref())
        .unwrap_or(default_target);

    // 3. Resolve release mode: CLI flag -> cached build.release -> default true
    let is_release = opts.release.unwrap_or(config.build.release);

    // 4. Resolve preset: CLI flag -> cached build.preset
    let preset = opts.preset.or(config.build.preset.as_deref());

    // 5. Auto-cache build settings in .goldsrc.toml if explicitly passed on CLI and changed
    let should_cache_backend = opts.backend.is_some() && config.build.backend != opts.backend;
    let should_cache_target =
        opts.target.is_some() && config.build.target.as_deref() != opts.target;
    let should_cache_release = opts.release.is_some() && config.build.release != is_release;
    let should_cache_preset =
        opts.preset.is_some() && config.build.preset.as_deref() != opts.preset;

    if should_cache_backend || should_cache_target || should_cache_release || should_cache_preset {
        let mut updated_config = config.clone();
        if opts.backend.is_some() {
            updated_config.build.backend = opts.backend;
        }
        if let Some(t) = opts.target {
            updated_config.build.target = Some(t.to_string());
        }
        if opts.release.is_some() {
            updated_config.build.release = is_release;
        }
        if let Some(p) = opts.preset {
            updated_config.build.preset = Some(p.to_string());
        }
        let config_path =
            LocalConfig::find_config_path().unwrap_or_else(|| PathBuf::from(".goldsrc.toml"));
        if let Err(e) = updated_config.save_cache(&config_path) {
            eprintln!(
                "Warning: Failed to update build cache in {}: {e}",
                config_path.display()
            );
        } else {
            println!("Cached build settings in {}", config_path.display());
        }
    }

    // 6. Build cargo command
    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--target").arg(target);

    if is_release {
        cmd.arg("--release");
    }

    // Handle backend compilation or plugin compilation
    let package_arg = if let Some(b) = effective_backend {
        let pkg_name = match b {
            BackendType::Metamod => "goldsrc-backend-metamod",
            BackendType::Standalone => "goldsrc-backend-standalone",
        };

        // If building backend, find runtime workspace manifest
        let runtime_manifest = find_runtime_manifest();
        if let Some(manifest) = runtime_manifest {
            cmd.arg("--manifest-path").arg(manifest);
        }
        cmd.arg("-p").arg(pkg_name);
        Some(pkg_name.to_string())
    } else if opts.workspace {
        cmd.arg("--workspace");
        None
    } else if let Some(pkg) = opts.package {
        cmd.arg("-p").arg(pkg);
        Some(pkg.to_string())
    } else {
        None
    };

    // Resolve debug and strip settings from preset or explicit flags
    let (resolved_debug, resolved_strip) = match preset {
        Some("production") => (Some("0"), Some("debuginfo")),
        Some("debug-symbols") => (Some("line-tables-only"), Some("none")),
        Some("dev") => (Some("2"), Some("none")),
        Some(unknown) => {
            return Err(format!(
                "Unknown preset '{unknown}'. Supported: 'production', 'debug-symbols', 'dev'"
            ));
        }
        None => (opts.debug_level, opts.strip),
    };

    let profile_name = if is_release { "release" } else { "dev" };

    if let Some(debug) = resolved_debug {
        cmd.arg("--config")
            .arg(format!("profile.{profile_name}.debug=\"{debug}\""));
    }

    if let Some(strip) = resolved_strip {
        cmd.arg("--config")
            .arg(format!("profile.{profile_name}.strip=\"{strip}\""));
    }

    println!(
        "Executing: cargo build --target {} {}{}{}",
        target,
        if is_release { "--release " } else { "" },
        if opts.workspace && effective_backend.is_none() {
            "--workspace ".to_string()
        } else {
            package_arg
                .as_deref()
                .map(|p| format!("-p {p} "))
                .unwrap_or_default()
        },
        if let Some(p) = preset {
            format!("[preset: {p}]")
        } else {
            String::new()
        }
    );

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to invoke cargo: {e}"))?;

    if status.success() {
        println!("Build completed successfully.");
        Ok(())
    } else {
        Err(format!("Cargo build exited with status: {status}"))
    }
}

/// Helper to locate `goldsrc-runtime/Cargo.toml` relative to current directory.
fn find_runtime_manifest() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("Cargo.toml"),
        PathBuf::from("../goldsrc-runtime/Cargo.toml"),
        PathBuf::from("../../goldsrc-runtime/Cargo.toml"),
        PathBuf::from("goldsrc-runtime/Cargo.toml"),
    ];
    for candidate in &candidates {
        if candidate.is_file() {
            let contains_target = std::fs::read_to_string(candidate)
                .map(|content| content.contains("goldsrc-backend-metamod"))
                .unwrap_or(false);
            if contains_target {
                return Some(candidate.clone());
            }
        }
    }
    None
}
