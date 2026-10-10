use crate::config::{BackendType, FilePatchRule, LocalConfig};
use crate::patch::{apply_key_value_patch, apply_list_entry_patch, evaluate_template};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Options for deploying plugins or runtime to a server.
pub struct DeployOptions<'a> {
    pub server_path: Option<&'a Path>,
    pub backend: Option<BackendType>,
    pub verify_only: bool,
    pub kill: bool,
    pub restart: bool,
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

    let should_kill = opts.kill || opts.restart || config.deploy.kill_server;
    let should_restart = opts.restart || config.deploy.restart_server;

    // Detect running HLDS process and capture its launch arguments if restart is requested
    let mut server_launch_info = None;
    if should_kill || should_restart {
        if let Some(proc_info) = find_running_hlds_process(&server_root) {
            println!(
                "Detected running HLDS process (PID: {}). Terminating for deployment...",
                proc_info.pid
            );
            terminate_process(proc_info.pid)?;
            // Small pause to allow OS file handles to be released
            std::thread::sleep(std::time::Duration::from_millis(500));
            server_launch_info = Some(proc_info);
        } else if opts.kill {
            println!(
                "No active HLDS process found matching {}",
                server_root.display()
            );
        }
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

    let backend = opts.backend.unwrap_or(config.deploy.backend);
    println!("Configured backend: {:?}", backend);

    let goldsrc_dir = match backend {
        BackendType::Metamod => target_mod_dir.join("addons").join("goldsrc"),
        BackendType::Standalone => target_mod_dir.join("goldsrc"),
    };
    let plugins_dir = goldsrc_dir.join("plugins");
    let lang_dir = goldsrc_dir.join("lang");

    println!("Deploy target: {}", goldsrc_dir.display());

    if opts.verify_only {
        println!("Verifying deployment structure...");
        if !goldsrc_dir.exists() {
            return Err(format!("{} directory not found", goldsrc_dir.display()));
        }
        println!("Verification OK: Deployment directory exists.");
        return Ok(());
    }

    fs::create_dir_all(&plugins_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&lang_dir).map_err(|e| e.to_string())?;

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

    if should_restart {
        restart_hlds_server(&server_root, mod_name, server_launch_info.as_ref())?;
    }

    Ok(())
}

/// Metadata about a running HLDS server process.
struct HldsProcessInfo {
    pid: u32,
    raw_cmdline: Option<String>,
}

/// Discovers a running HLDS process whose executable path is inside `server_root`.
fn find_running_hlds_process(server_root: &Path) -> Option<HldsProcessInfo> {
    if cfg!(windows) {
        // Query CIM/WMI on Windows
        let server_canonical = server_root
            .canonicalize()
            .unwrap_or_else(|_| server_root.to_path_buf());
        let server_str = server_canonical.to_string_lossy().to_lowercase();

        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_Process | Where-Object Name -eq 'hlds.exe' | Select-Object ProcessId, ExecutablePath, CommandLine | ConvertTo-Json -Compress",
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let json_str = String::from_utf8(output.stdout).ok()?;
        if json_str.trim().is_empty() {
            return None;
        }

        #[derive(serde::Deserialize)]
        struct WinProc {
            #[serde(rename = "ProcessId")]
            process_id: u32,
            #[serde(rename = "ExecutablePath")]
            executable_path: Option<String>,
            #[serde(rename = "CommandLine")]
            command_line: Option<String>,
        }

        // Output can be a single object or an array of objects
        let procs: Vec<WinProc> = if let Ok(single) = serde_json::from_str::<WinProc>(&json_str) {
            vec![single]
        } else {
            serde_json::from_str::<Vec<WinProc>>(&json_str).unwrap_or_default()
        };

        for p in procs {
            if let Some(exe) = &p.executable_path {
                let exe_lower = exe.to_lowercase();
                if exe_lower.contains(&server_str) || exe_lower.contains("desktop\\server") {
                    return Some(HldsProcessInfo {
                        pid: p.process_id,
                        raw_cmdline: p.command_line,
                    });
                }
            }
        }
    } else {
        // Unix: use pgrep / ps
        let output = Command::new("pgrep")
            .arg("-f")
            .arg("hlds_linux")
            .output()
            .ok()?;

        if output.status.success() {
            let pid_str = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = pid_str.lines().next() {
                let parsed_pid = first_line.trim().parse::<u32>().ok();
                if let Some(pid) = parsed_pid {
                    return Some(HldsProcessInfo {
                        pid,
                        raw_cmdline: None,
                    });
                }
            }
        }
    }

    None
}

/// Terminates a process by its PID.
fn terminate_process(pid: u32) -> Result<(), String> {
    if cfg!(windows) {
        let status = Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .status()
            .map_err(|e| format!("Failed to invoke taskkill: {e}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("taskkill failed with status: {status}"))
        }
    } else {
        let status = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()
            .map_err(|e| format!("Failed to invoke kill: {e}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("kill failed with status: {status}"))
        }
    }
}

/// Restarts the HLDS server process.
fn restart_hlds_server(
    server_root: &Path,
    mod_name: &str,
    info: Option<&HldsProcessInfo>,
) -> Result<(), String> {
    let exe_name = if cfg!(windows) {
        "hlds.exe"
    } else {
        "hlds_run"
    };
    let hlds_exe = server_root.join(exe_name);

    if !hlds_exe.exists() {
        return Err(format!(
            "Server executable not found at {}",
            hlds_exe.display()
        ));
    }

    println!("Restarting server: {} ...", hlds_exe.display());

    let final_args: Vec<String> = if let Some(raw) = info.and_then(|i| i.raw_cmdline.as_deref()) {
        let args = parse_command_line_args(raw);
        if args.len() > 1 {
            args[1..].to_vec()
        } else {
            vec![
                "-game".into(),
                mod_name.into(),
                "-console".into(),
                "-insecure".into(),
                "+maxplayers".into(),
                "32".into(),
                "+map".into(),
                "de_dust2".into(),
                "+port".into(),
                "27015".into(),
            ]
        }
    } else {
        vec![
            "-game".into(),
            mod_name.into(),
            "-console".into(),
            "-insecure".into(),
            "+maxplayers".into(),
            "32".into(),
            "+map".into(),
            "de_dust2".into(),
            "+port".into(),
            "27015".into(),
        ]
    };

    println!("Server launch arguments: {:?}", final_args);

    if cfg!(windows) {
        // On Windows, launching directly via Command::spawn binds HLDS to the parent console/job object.
        // If grs was run from an IDE/terminal, terminating or exiting grs may kill HLDS or leave it without a console.
        // Using WMI Win32_Process::Create spawns HLDS independently under WmiPrvSE (session 1), completely decoupled.
        let mut full_cmdline = format!("\"{}\"", hlds_exe.display());
        for arg in &final_args {
            if arg.contains(' ') {
                full_cmdline.push_str(&format!(" \"{arg}\""));
            } else {
                full_cmdline.push_str(&format!(" {arg}"));
            }
        }

        let cur_dir = server_root.display().to_string();
        let ps_script = format!(
            "Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{{ CommandLine = '{}'; CurrentDirectory = '{}' }}",
            full_cmdline.replace('\'', "''"),
            cur_dir.replace('\'', "''")
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .output()
            .map_err(|e| format!("Failed to spawn HLDS via WMI: {e}"))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to spawn HLDS via WMI: {err}"));
        }
    } else {
        let mut cmd = Command::new(&hlds_exe);
        cmd.current_dir(server_root);
        cmd.args(&final_args);

        cmd.spawn()
            .map_err(|e| format!("Failed to spawn {}: {e}", hlds_exe.display()))?;
    }

    println!("Server process launched successfully.");
    Ok(())
}

/// Basic commandline tokenizer respecting quotes.
fn parse_command_line_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for c in input.chars() {
        match c {
            '"' => in_quotes = !in_quotes,
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}
