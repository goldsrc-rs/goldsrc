//! Build orchestration for WASM plugins and engine backends (`grs build`).

use std::process::Command;

/// Options for building GoldSrc plugins or backends.
pub struct BuildOptions<'a> {
    pub package: Option<&'a str>,
    pub target: Option<&'a str>,
    pub release: bool,
}

pub fn execute_build(opts: BuildOptions<'_>) -> Result<(), String> {
    let target = opts.target.unwrap_or("wasm32-unknown-unknown");

    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--target").arg(target);

    if opts.release {
        cmd.arg("--release");
    }

    if let Some(pkg) = opts.package {
        cmd.arg("-p").arg(pkg);
    }

    println!(
        "Executing: cargo build --target {} {}{}",
        target,
        if opts.release { "--release " } else { "" },
        opts.package.map(|p| format!("-p {p}")).unwrap_or_default()
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
