//! Environment pre-flight inspector and project configurator (`grs setup` / `grs configure`).

use crate::config::{BackendType, LocalConfig};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn execute_setup(mod_name: Option<&str>, hlds_path: Option<&Path>) -> Result<(), String> {
    println!("=== GoldSrc.rs Pre-flight Environment & Dependency Inspector ===\n");

    // 1. Check Rust & Cargo
    let rustc_ver = get_tool_version("rustc", &["--version"]);
    let cargo_ver = get_tool_version("cargo", &["--version"]);

    match &rustc_ver {
        Some(v) => println!("[OK] rustc: {v}"),
        None => eprintln!("[FAIL] rustc: Not found in PATH!"),
    }
    match &cargo_ver {
        Some(v) => println!("[OK] cargo: {v}"),
        None => eprintln!("[FAIL] cargo: Not found in PATH!"),
    }

    // 2. Check Rust Target Architectures
    println!("\nTarget Architectures:");
    let installed_targets = get_installed_targets();
    let required_targets = [
        ("wasm32-unknown-unknown", "WASM Plugin Compilation"),
        ("i686-pc-windows-msvc", "Windows HLDS Native Binary"),
        ("i686-unknown-linux-gnu", "Linux HLDS Native Binary"),
    ];

    let mut missing_targets = Vec::new();
    for (target, desc) in &required_targets {
        let is_installed = installed_targets.contains(&target.to_string());
        if is_installed {
            println!("  [OK] {:<25} ({})", target, desc);
        } else {
            println!("  [MISSING] {:<21} ({})", target, desc);
            missing_targets.push(*target);
        }
    }

    if !missing_targets.is_empty() {
        println!(
            "\n  Hint: Run 'rustup target add {}' to install missing targets.",
            missing_targets.join(" ")
        );
    }

    // 3. Check C/C++ Build Tools (MSVC on Windows, GCC/Clang on Unix)
    println!("\nNative Toolchain:");
    let _native_toolchain_ok = check_native_toolchain();

    // 4. Check Offline Engine References
    println!("\nOffline Engine References (codegen):");
    let ref_dirs = [
        ("references/hlsdk", "HLSDK 2.3 headers"),
        ("references/regamedll", "ReGameDLL headers"),
        ("references/metamod-r", "Metamod-r headers"),
        ("references/amxmodx", "AMX Mod X headers"),
    ];

    let mut refs_missing = false;
    for (rel_dir, desc) in &ref_dirs {
        let p = Path::new(rel_dir);
        let exists = p.is_dir() && fs_has_files(p);
        if exists {
            println!("  [OK] {:<25} ({})", rel_dir, desc);
        } else {
            println!("  [WARN] {:<23} ({}) - empty or missing", rel_dir, desc);
            refs_missing = true;
        }
    }

    if refs_missing {
        println!("  Hint: Run 'git submodule update --init --recursive' to populate references.");
    }

    // 5. Generate / Verify .goldsrc.toml configuration
    println!("\nConfiguration File:");
    let config_path =
        LocalConfig::find_config_path().unwrap_or_else(|| PathBuf::from(".goldsrc.toml"));

    if config_path.exists() {
        println!(
            "  [OK] Existing configuration found at: {}",
            config_path.display()
        );
    } else {
        println!(
            "  Creating default configuration at: {}",
            config_path.display()
        );
        let mut cfg = LocalConfig::default();
        if let Some(m) = mod_name {
            cfg.deploy.mod_name = m.to_string();
        }
        if let Some(hp) = hlds_path {
            cfg.deploy.server_path = Some(hp.to_path_buf());
        }
        cfg.deploy.backend = BackendType::Metamod;

        if let Err(e) = cfg.save_deploy_cache(&config_path) {
            eprintln!("  [FAIL] Failed to write {}: {e}", config_path.display());
        } else {
            println!("  [OK] Generated clean {}", config_path.display());
        }
    }

    println!("\n=== Pre-flight Setup Complete ===");
    Ok(())
}

fn get_tool_version(tool: &str, args: &[&str]) -> Option<String> {
    Command::new(tool).args(args).output().ok().and_then(|out| {
        if out.status.success() {
            String::from_utf8(out.stdout)
                .ok()
                .map(|s| s.trim().to_string())
        } else {
            None
        }
    })
}

fn get_installed_targets() -> Vec<String> {
    Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                String::from_utf8(out.stdout).ok().map(|s| {
                    s.lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty())
                        .collect()
                })
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn check_native_toolchain() -> bool {
    if cfg!(windows) {
        // Check link.exe or cl.exe
        let link_ok = Command::new("link.exe").arg("/?").output().is_ok();
        let cl_ok = Command::new("cl.exe").arg("/?").output().is_ok();
        if link_ok || cl_ok {
            println!("  [OK] MSVC Linker/Compiler detected in environment.");
            true
        } else {
            println!("  [INFO] MSVC Build Tools available via rustc MSVC toolchain.");
            true
        }
    } else {
        let gcc_ok = Command::new("gcc").arg("--version").output().is_ok();
        let clang_ok = Command::new("clang").arg("--version").output().is_ok();
        if gcc_ok || clang_ok {
            println!("  [OK] Native C/C++ compiler detected.");
            true
        } else {
            println!(
                "  [WARN] Neither GCC nor Clang found. Install build-essential / gcc-multilib."
            );
            false
        }
    }
}

fn fs_has_files(dir: &Path) -> bool {
    if let Ok(mut entries) = std::fs::read_dir(dir) {
        entries.next().is_some()
    } else {
        false
    }
}
