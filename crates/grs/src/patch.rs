//! Declarative file patch engine and template substitution for server deployment.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Evaluates a template string by replacing `{var}` placeholders with values from `vars`.
pub fn evaluate_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (k, v) in vars {
        let needle = format!("{{{k}}}");
        result = result.replace(&needle, v);
    }
    result
}

/// Applies a list entry patch to a file (e.g. `plugins.ini`).
///
/// 1. If line matching `match_pattern` is found and commented out (starts with any in `comment_prefixes`),
///    it uncomments/replaces it with `new_entry`.
/// 2. If active line is found, keeps/updates it.
/// 3. If line is not found, appends `new_entry` to the file.
pub fn apply_list_entry_patch(
    file_path: &Path,
    new_entry: &str,
    match_pattern: &str,
    comment_prefixes: &[String],
) -> Result<bool, String> {
    let content = if file_path.exists() {
        fs::read_to_string(file_path).map_err(|e| e.to_string())?
    } else {
        String::new()
    };

    let mut found = false;
    let mut modified = false;
    let mut new_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Check if line contains the match pattern (whether commented or active)
        if trimmed.contains(match_pattern) {
            found = true;
            let mut is_commented = false;
            for prefix in comment_prefixes {
                if trimmed.starts_with(prefix) {
                    is_commented = true;
                    break;
                }
            }

            if is_commented || trimmed != new_entry {
                new_lines.push(new_entry.to_string());
                modified = true;
            } else {
                new_lines.push(line.to_string());
            }
        } else {
            new_lines.push(line.to_string());
        }
    }

    if !found {
        new_lines.push(new_entry.to_string());
        modified = true;
    }

    if modified || !file_path.exists() {
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let output = if new_lines.is_empty() {
            String::new()
        } else {
            format!("{}\r\n", new_lines.join("\r\n"))
        };
        fs::write(file_path, output).map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Applies a key-value patch to a file (e.g. `liblist.gam`).
///
/// Finds key prefix, replaces value, or appends key-value if missing.
pub fn apply_key_value_patch(
    file_path: &Path,
    key: &str,
    value: &str,
    separator: &str,
) -> Result<bool, String> {
    let content = if file_path.exists() {
        fs::read_to_string(file_path).map_err(|e| e.to_string())?
    } else {
        String::new()
    };

    let mut replaced = false;
    let mut modified = false;
    let mut new_lines = Vec::new();
    let expected_line = format!("{key}{separator}{value}");

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(key) {
            replaced = true;
            if trimmed != expected_line {
                new_lines.push(expected_line.clone());
                modified = true;
            } else {
                new_lines.push(line.to_string());
            }
        } else {
            new_lines.push(line.to_string());
        }
    }

    if !replaced {
        new_lines.push(expected_line);
        modified = true;
    }

    if modified || !file_path.exists() {
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let output = if new_lines.is_empty() {
            String::new()
        } else {
            format!("{}\r\n", new_lines.join("\r\n"))
        };
        fs::write(file_path, output).map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_template() {
        let mut vars = HashMap::new();
        vars.insert("platform".to_string(), "win32".to_string());
        vars.insert("lib_ext".to_string(), "dll".to_string());

        let res = evaluate_template("{platform} addons/goldsrc/bin/test.{lib_ext}", &vars);
        assert_eq!(res, "win32 addons/goldsrc/bin/test.dll");
    }

    #[test]
    fn test_list_entry_uncomment() {
        let dir = std::env::temp_dir().join(format!("test_patch_list_{}", std::process::id()));
        let file = dir.join("plugins.ini");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            &file,
            "; win32 addons/goldsrc/lib/goldsrc_metamod.dll\r\nother_plugin.dll\r\n",
        )
        .unwrap();

        let prefixes = vec![";".to_string(), "//".to_string()];
        let changed = apply_list_entry_patch(
            &file,
            "win32 addons/goldsrc/lib/goldsrc_metamod.dll",
            "goldsrc_metamod",
            &prefixes,
        )
        .unwrap();

        assert!(changed);
        let res = fs::read_to_string(&file).unwrap();
        assert!(res.contains("win32 addons/goldsrc/lib/goldsrc_metamod.dll\r\nother_plugin.dll"));
        assert!(!res.contains("; win32"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_key_value_patch() {
        let dir = std::env::temp_dir().join(format!("test_patch_kv_{}", std::process::id()));
        let file = dir.join("liblist.gam");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            &file,
            "game \"Counter-Strike\"\r\ngamedll \"dlls\\mp.dll\"\r\n",
        )
        .unwrap();

        let changed =
            apply_key_value_patch(&file, "gamedll", "\"addons\\metamod\\metamod.dll\"", " ")
                .unwrap();

        assert!(changed);
        let res = fs::read_to_string(&file).unwrap();
        assert!(res.contains("gamedll \"addons\\metamod\\metamod.dll\""));
        assert!(!res.contains("dlls\\mp.dll"));

        let _ = fs::remove_dir_all(dir);
    }
}
