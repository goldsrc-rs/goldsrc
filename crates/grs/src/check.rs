//! Pre-commit and code quality validation orchestrator (`grs check`).

use std::process::Command;

pub fn execute_check() -> Result<(), String> {
    println!("Running code quality and pre-commit checks...");

    // 1. Cargo fmt
    println!("[1/3] Checking code formatting (cargo fmt --check)...");
    let fmt_status = Command::new("cargo")
        .args(["fmt", "--check"])
        .status()
        .map_err(|e| format!("Failed to run cargo fmt: {e}"))?;

    if !fmt_status.success() {
        return Err("Code formatting check failed. Run 'cargo fmt' to fix.".to_string());
    }

    // 2. Cargo clippy
    println!("[2/3] Running linter (cargo clippy --workspace)...");
    let clippy_status = Command::new("cargo")
        .args(["clippy", "--workspace", "--", "-D", "warnings"])
        .status()
        .map_err(|e| format!("Failed to run cargo clippy: {e}"))?;

    if !clippy_status.success() {
        return Err("Clippy lint checks failed.".to_string());
    }

    // 3. Cargo test
    println!("[3/3] Running tests (cargo test --workspace)...");
    let test_status = Command::new("cargo")
        .args(["test", "--workspace"])
        .status()
        .map_err(|e| format!("Failed to run cargo test: {e}"))?;

    if !test_status.success() {
        return Err("Workspace tests failed.".to_string());
    }

    println!("All quality checks passed successfully!");
    Ok(())
}
