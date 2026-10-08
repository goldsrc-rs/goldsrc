//! Build orchestration for WASM plugins and engine backends (`grs build`).

use std::process::Command;

/// Options for building GoldSrc plugins or backends.
pub struct BuildOptions<'a> {
    pub package: Option<&'a str>,
    pub target: Option<&'a str>,
    pub workspace: bool,
    pub release: bool,
    pub preset: Option<&'a str>,
    pub debug_level: Option<&'a str>,
    pub strip: Option<&'a str>,
}

pub fn execute_build(opts: BuildOptions<'_>) -> Result<(), String> {
    let target = opts.target.unwrap_or("wasm32-unknown-unknown");

    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--target").arg(target);

    if opts.release {
        cmd.arg("--release");
    }

    if opts.workspace {
        cmd.arg("--workspace");
    } else if let Some(pkg) = opts.package {
        cmd.arg("-p").arg(pkg);
    }

    // Resolve debug and strip settings from preset or explicit flags
    let (resolved_debug, resolved_strip) = match opts.preset {
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

    let profile_name = if opts.release { "release" } else { "dev" };

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
        if opts.release { "--release " } else { "" },
        if opts.workspace {
            "--workspace ".to_string()
        } else {
            opts.package.map(|p| format!("-p {p} ")).unwrap_or_default()
        },
        if let Some(preset) = opts.preset {
            format!("[preset: {preset}]")
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
