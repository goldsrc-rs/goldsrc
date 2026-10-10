//! GoldSrc Developer CLI (`grs`).

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

mod arch;
mod build;
mod check;
mod config;
mod deploy;
mod patch;
mod scaffold;
mod setup;

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
    /// Inspect environment, toolchains, offline references, and configure project
    #[command(alias = "configure")]
    Setup {
        /// Target game mod name (e.g. 'cstrike')
        #[arg(long)]
        game: Option<String>,
        /// Path to HLDS server directory
        #[arg(long)]
        path: Option<PathBuf>,
    },
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
        /// Engine runtime backend to build ('metamod' or 'standalone')
        #[arg(long, value_enum)]
        backend: Option<BackendArg>,
        /// Package name to build (-p <pkg>)
        #[arg(short, long)]
        package: Option<String>,
        /// Target architecture triple (e.g. wasm32-unknown-unknown, i686-pc-windows-msvc)
        #[arg(long)]
        target: Option<String>,
        /// Build all packages in the workspace (--workspace)
        #[arg(long)]
        workspace: bool,
        /// Build in release mode
        #[arg(long)]
        release: Option<bool>,
        /// Build preset: 'production', 'debug-symbols', or 'dev'
        #[arg(long)]
        preset: Option<String>,
        /// Override profile.release.debug (e.g. '0', '1', '2', 'line-tables-only')
        #[arg(long)]
        debug_level: Option<String>,
        /// Override profile.release.strip (e.g. 'none', 'debuginfo', 'symbols')
        #[arg(long)]
        strip: Option<String>,
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
    /// Sewing Machine Architecture (SMA) and Architecture-as-Code tooling
    Arch {
        #[command(subcommand)]
        sub: arch::ArchCommands,
    },
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
        Commands::Setup { game, path } => {
            if let Err(err) = setup::execute_setup(game.as_deref(), path.as_deref()) {
                eprintln!("Setup error: {err}");
                std::process::exit(1);
            }
        }
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
            backend,
            package,
            target,
            workspace,
            release,
            preset,
            debug_level,
            strip,
        } => {
            let opts = build::BuildOptions {
                backend: backend.map(Into::into),
                package: package.as_deref(),
                target: target.as_deref(),
                workspace,
                release,
                preset: preset.as_deref(),
                debug_level: debug_level.as_deref(),
                strip: strip.as_deref(),
            };
            if let Err(err) = build::execute_build(opts, &local_cfg) {
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
        Commands::Arch { sub } => {
            if let Err(err) = arch::execute_arch(sub) {
                eprintln!("Architecture check error: {err}");
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
