//! Architecture-as-Code runner (`grs arch`).
//!
//! Exposes Sewing Machine Architecture (SMA) static analysis, graph extraction,
//! and Scrooge systems metrics via `stitch-cli`.

use clap::Subcommand;
use std::path::{Path, PathBuf};
use stitch_cli::check::CheckRunner;
use stitch_cli::config::{RuleSeverity, StitchConfig};
use stitch_cli::graph::GraphExtractor;
use stitch_cli::metrics::MetricsAuditor;

#[derive(Subcommand, Debug)]
pub enum ArchCommands {
    /// Fast pre-build AST scanner: validates SMA taxonomy, Scrooge rules, and boundaries
    Check {
        /// Target workspace directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
        /// Treat warnings as hard errors
        #[arg(long)]
        strict: bool,
    },
    /// Extracts architectural DAG: outputs Mermaid, Graphviz DOT, JSON, or interactive HTML
    Graph {
        /// Target workspace directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
        /// Output format (text, json, mermaid, dot, html)
        #[arg(short, long, default_value = "mermaid")]
        format: String,
        /// Optional destination file to write output to
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Audits L1D cache-line alignment, struct padding holes, and optimal field ordering
    Metrics {
        /// Target workspace directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },
}

pub fn execute_arch(cmd: ArchCommands) -> Result<(), String> {
    match cmd {
        ArchCommands::Check { dir, strict } => run_check(&dir, strict),
        ArchCommands::Graph {
            dir,
            format,
            output,
        } => run_graph(&dir, &format, output),
        ArchCommands::Metrics { dir } => run_metrics(&dir),
    }
}

pub fn run_check(target_dir: &Path, strict: bool) -> Result<(), String> {
    println!(
        "==> [SMA] Scanning workspace architecture at `{}`...",
        target_dir.display()
    );
    let config = StitchConfig::load(target_dir);
    let runner = CheckRunner::new(&config, target_dir);
    let diagnostics = runner.run();

    let mut error_count = 0;
    let mut warn_count = 0;

    for diag in &diagnostics {
        match diag.severity {
            RuleSeverity::Deny => {
                error_count += 1;
                eprint!("{}", diag.render_rustc());
            }
            RuleSeverity::Warn => {
                warn_count += 1;
                if strict {
                    error_count += 1;
                    eprint!("{}", diag.render_rustc());
                } else {
                    println!("{}", diag.render_rustc());
                }
            }
            RuleSeverity::Allow => {}
        }
    }

    if error_count > 0 {
        Err(format!(
            "SMA architectural check failed: {error_count} error(s), {warn_count} warning(s)."
        ))
    } else {
        println!(
            "\n✅ SMA Architecture Clean: 0 errors, {warn_count} warning(s) found."
        );
        Ok(())
    }
}

pub fn run_graph(
    target_dir: &Path,
    format: &str,
    output_file: Option<PathBuf>,
) -> Result<(), String> {
    let extractor = GraphExtractor::new(target_dir);
    let graph = extractor.extract();

    let output_str = match format {
        "json" => serde_json::to_string_pretty(&graph).unwrap_or_default(),
        "mermaid" => graph.to_mermaid(),
        "dot" => graph.to_dot(),
        "html" => graph.to_html(),
        _ => graph.to_mermaid(),
    };

    if let Some(path) = output_file {
        std::fs::write(&path, &output_str)
            .map_err(|e| format!("Failed to write graph to `{}`: {e}", path.display()))?;
        println!(
            "✅ Architecture DAG exported to `{}` (format: {}).",
            path.display(),
            format
        );
    } else {
        println!("{output_str}");
    }

    Ok(())
}

pub fn run_metrics(target_dir: &Path) -> Result<(), String> {
    println!("==> [SMA] Running Scrooge Systems Memory & Alignment Audit...");
    let auditor = MetricsAuditor::new(target_dir);
    let reports = auditor.audit();

    println!("\n{:=<80}", "");
    println!("                    SCROOGE SYSTEMS MEMORY & ALIGNMENT AUDIT");
    println!("{:=<80}", "");

    for r in &reports {
        if r.total_padding > 0 || r.crosses_cache_line {
            println!("\n[STRUCT] {} ({}:{})", r.name, r.file, r.line);
            println!("  Declared Size:   {} bytes", r.total_size);
            println!("  Padding Waste:   {} bytes", r.total_padding);
            println!(
                "  Optimal Waste:   {} bytes (via descending alignment order)",
                r.optimal_padding
            );
            if r.crosses_cache_line {
                println!("  Cache Status:    CROSSES 64-byte L1D boundary!");
            }

            for f in &r.fields {
                if f.padding_before > 0 {
                    println!(
                        "    +{:02} [PADDING HOLE] ({} bytes)",
                        f.offset - f.padding_before,
                        f.padding_before
                    );
                }
                println!(
                    "    +{:02} {}: {} ({} B, align {})",
                    f.offset, f.name, f.type_str, f.size, f.align
                );
            }
        }
    }
    println!("\n{:=<80}", "");
    println!("Audit completed across {} structs.", reports.len());
    Ok(())
}
