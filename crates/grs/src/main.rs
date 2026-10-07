//! GoldSrc Developer CLI (`grs`).

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "grs")]
#[command(about = "GoldSrc Developer CLI: build, test, package, and inspect GoldSrc plugins and runtime", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect plugins or runtime state
    Pl {
        #[command(subcommand)]
        sub: PlCommands,
    },
    /// Validate bundle.toml and manifests
    Check,
}

#[derive(Subcommand)]
enum PlCommands {
    /// Show plugin information
    Info {
        id: String,
        #[arg(short, long)]
        field: Option<String>,
    },
    /// Inspect plugin memory allocation telemetry
    Memory,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pl { sub } => match sub {
            PlCommands::Info { id, field } => {
                println!("Inspecting plugin '{}', field: {:?}", id, field);
            }
            PlCommands::Memory => {
                println!("Plugin memory telemetry: active");
            }
        },
        Commands::Check => {
            println!("Validating GoldSrc configuration...");
        }
    }
}
