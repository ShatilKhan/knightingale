use clap::{Parser, Subcommand};
use knightingale_core::ipc::{self, Request, Response};

mod style;

#[derive(Parser)]
#[command(
    name = "knightingale",
    version,
    about = "Voice dictation daemon (CLI client)",
    long_about = None,
    styles = style::clap_styles(),
)]
struct Cli {
    /// Suppress the ASCII banner.
    #[arg(long, global = true)]
    no_banner: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Toggle recording on/off.
    Toggle,
    /// Print daemon status.
    Status,
    /// Tail the daemon log.
    Logs {
        #[arg(
            long,
            value_name = "DURATION",
            help = "filter to last duration, e.g. 1h, 30m"
        )]
        since: Option<String>,
        #[arg(long, help = "print log file path and exit")]
        path: bool,
    },
    /// Manage configuration.
    Config(ConfigArgs),
    /// Diagnose problems.
    Doctor,
    /// Re-run the interactive setup flow.
    Setup,
    /// Manage local Whisper models.
    Model {
        #[command(subcommand)]
        cmd: ModelCmd,
    },
    /// Run the eval harness against a corpus directory.
    Eval {
        #[arg(
            long,
            value_name = "DIR",
            help = "directory with .wav and matching .txt files"
        )]
        corpus: std::path::PathBuf,
    },
    /// Print the banner.
    Banner {
        #[arg(long, help = "use the wide variant")]
        wide: bool,
    },
}

#[derive(clap::Args)]
struct ConfigArgs {
    #[command(subcommand)]
    cmd: ConfigCmd,
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// Print current configuration (secrets redacted).
    Show,
    /// Open the config TOML in $EDITOR.
    Edit,
    /// Open the .env secrets file in $EDITOR.
    EditSecrets,
    /// Test the active provider's auth + reachability.
    Test,
    /// List supported providers and their default models.
    Provider {
        #[command(subcommand)]
        cmd: ProviderCmd,
    },
    /// Manage microphone devices.
    Mic {
        #[command(subcommand)]
        cmd: MicCmd,
    },
    /// Set a config field.
    Set { key: String, value: String },
    /// Set an API key (hidden prompt).
    SetKey { provider: String },
}

#[derive(Subcommand)]
enum ProviderCmd {
    List,
}

#[derive(Subcommand)]
enum ModelCmd {
    /// List installed + available local models.
    List,
    /// Download a model by alias.
    Pull { alias: String },
    /// Print the recommended model for this machine.
    Recommend {
        #[arg(long)]
        english_only: bool,
    },
}

#[derive(Subcommand)]
enum MicCmd {
    List,
    Set { name: String },
}

fn main() -> miette::Result<()> {
    knightingale_core::load_env();
    let cli = Cli::parse();
    if !cli.no_banner && std::env::var("KNIGHTINGALE_NO_BANNER").is_err() {
        eprintln!("{}", style::BANNER);
    }
    match cli.cmd {
        Cmd::Toggle => toggle(),
        Cmd::Status => status(),
        Cmd::Logs { since, path } => commands::logs(since, path),
        Cmd::Doctor => commands::doctor(),
        Cmd::Setup => commands::setup(),
        Cmd::Model { cmd } => match cmd {
            ModelCmd::List => commands::model_list(),
            ModelCmd::Pull { alias } => commands::model_pull(&alias),
            ModelCmd::Recommend { english_only } => commands::model_recommend(english_only),
        },
        Cmd::Eval { corpus } => commands::eval_run(corpus),
        Cmd::Banner { wide } => banner(wide),
        Cmd::Config(args) => match args.cmd {
            ConfigCmd::Show => commands::config_show(),
            ConfigCmd::Edit => commands::config_edit(false),
            ConfigCmd::EditSecrets => commands::config_edit(true),
            ConfigCmd::Test => commands::config_test(),
            ConfigCmd::Provider { cmd } => match cmd {
                ProviderCmd::List => commands::provider_list(),
            },
            ConfigCmd::Mic { cmd } => match cmd {
                MicCmd::List => commands::mic_list(),
                MicCmd::Set { name } => commands::mic_set(name),
            },
            ConfigCmd::Set { key, value } => commands::config_set(key, value),
            ConfigCmd::SetKey { provider } => commands::config_set_key(provider),
        },
    }
}

fn toggle() -> miette::Result<()> {
    let resp = ipc::send(&Request::Toggle).map_err(miette::Report::from)?;
    if let Response::Err { message } = resp {
        Err(miette::miette!(message))
    } else {
        Ok(())
    }
}

fn status() -> miette::Result<()> {
    let resp = ipc::send(&Request::Status).map_err(miette::Report::from)?;
    match resp {
        Response::Ok { message } => {
            println!("{}", message.unwrap_or_else(|| "ok".into()));
            Ok(())
        }
        Response::Err { message } => Err(miette::miette!(message)),
    }
}

fn banner(wide: bool) -> miette::Result<()> {
    println!(
        "{}",
        if wide {
            style::BANNER_WIDE
        } else {
            style::BANNER
        }
    );
    Ok(())
}

mod commands;
