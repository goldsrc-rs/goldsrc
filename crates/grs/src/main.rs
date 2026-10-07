//! GoldSrc Developer CLI (`grs`).

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

mod build;
mod check;
mod config;
mod deploy;
mod scaffold;

use config::{BackendType, LocalConfig};

#[derive(Parser)]
#[command(name = "grs")]
#[command(about = "GoldSrc Developer CLI: build, test, package, deploy, and scaffold GoldSrc plugins and runtime", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold a new GoldSrc WebAssembly plugin
    New {
        /// Plugin name / folder to create
        name: String,
        /// Target game SDK (default: engine pure/agnostic, or 'cstrike')
        #[arg(long)]
        game: Option<String>,
        /// Scaffolding template
        #[arg(long, default_value = "minimal")]
        template: String,
    },
    /// Build WebAssembly plugins or runtime backends
    Build {
        /// Package name to build (-p <pkg>)
        #[arg(short, long)]
        package: Option<String>,
        /// Target architecture triple (default: wasm32-unknown-unknown)
        #[arg(long)]
        target: Option<String>,
        /// Build in release mode
        #[arg(long, default_value_t = true)]
        release: bool,
    },
    /// Deploy plugins and runtime to a local HLDS / ReHLDS test server
    Deploy {
        /// Path to HLDS server directory (overrides .local.toml)
        #[arg(long)]
        path: Option<PathBuf>,
        /// Engine backend to deploy
        #[arg(long, value_enum)]
        backend: Option<BackendArg>,
        /// Verify target directory structure without copying files
        #[arg(long)]
        verify: bool,
    },
    /// Validate workspace code formatting, lints, and unit tests
    Check,
    /// Inspect plugins or runtime state
    Pl {
        #[command(subcommand)]
        sub: PlCommands,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum BackendArg {
    Metamod,
    Standalone,
}

impl From<BackendArg> for BackendType {
    fn from(arg: BackendArg) -> Self {
        match arg {
            BackendArg::Metamod => BackendType::Metamod,
            BackendArg::Standalone => BackendType::Standalone,
        }
    }
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
    let local_cfg = LocalConfig::load();

    match cli.command {
        Commands::New {
            name,
            game,
            template,
        } => {
            let opts = scaffold::NewOptions {
                name: &name,
                game: game.as_deref(),
                template: &template,
            };
            match scaffold::scaffold_plugin(opts) {
                Ok(path) => println!("Created new plugin at: {}", path.display()),
                Err(err) => eprintln!("Error creating plugin: {err}"),
            }
        }
        Commands::Build {
            package,
            target,
            release,
        } => {
            let opts = build::BuildOptions {
                package: package.as_deref(),
                target: target.as_deref(),
                release,
            };
            if let Err(err) = build::execute_build(opts) {
                eprintln!("Build error: {err}");
                std::process::exit(1);
            }
        }
        Commands::Deploy {
            path,
            backend,
            verify,
        } => {
            let opts = deploy::DeployOptions {
                server_path: path.as_deref(),
                backend: backend.map(Into::into),
                verify_only: verify,
            };
            if let Err(err) = deploy::execute_deploy(opts, &local_cfg) {
                eprintln!("Deploy error: {err}");
                std::process::exit(1);
            }
        }
        Commands::Check => {
            if let Err(err) = check::execute_check() {
                eprintln!("Check error: {err}");
                std::process::exit(1);
            }
        }
        Commands::Pl { sub } => match sub {
            PlCommands::Info { id, field } => {
                println!("Inspecting plugin '{id}', field: {field:?}");
            }
            PlCommands::Memory => {
                println!("Plugin memory telemetry: active");
            }
        },
    }
}
