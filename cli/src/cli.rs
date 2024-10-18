use clap::{Parser, Subcommand, ValueHint};

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub subcommands: Option<SubCommands>,

    #[arg(long,
    env = "MONEDERO_CONFIG_PATH",
    value_hint = ValueHint::FilePath,
    value_name = "FILEPATH",
    required = false,
    global = true,
    )]
    pub config: Option<std::path::PathBuf>,

    #[arg(
        long,
        short,
        env = "MONEDERO_PROFILE",
        global = true,
        value_name = "PROFILE_NAME",
        help = "override default config with this profile name"
    )]
    pub profile: Option<String>,

    #[arg(
        long,
        env = "MONEDERO_MAINNET",
        global = true,
        help = "use mainnet (default is testnet/devnet)"
    )]
    pub mainnet: bool,
}

#[derive(Debug, Subcommand)]
pub enum SubCommands {
    #[command()]
    Init,

    #[command()]
    Version {},
}
