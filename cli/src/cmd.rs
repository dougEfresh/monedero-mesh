use clap::{Args, Parser, Subcommand, ValueHint};
mod transfer;

pub use transfer::*;

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

    #[arg(
        long,
        env = "MONEDERO_MAX_FEE",
        default_value_t = 50000,
        global = true,
        help = "max fee for compute budget program in micro-lamports"
    )]
    pub max_fee: u64,

    #[arg(
        long,
        env = "MONEDERO_DEFAULT_MEMO",
        global = true,
        help = "memo to add for every transaction"
    )]
    pub default_memo: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum SubCommands {
    #[command()]
    Fees,
    #[command()]
    Balance,
    #[command()]
    Init,
    #[command()]
    Pair,
    #[command()]
    Version {},
    #[command()]
    Transfer(TransferArgs),
    #[command()]
    Stake(StakeArgs),
}

#[derive(Debug, Args)]
pub struct StakeArgs {
    #[command(subcommand)]
    pub command: StakeCommand,
}

#[derive(Debug, Subcommand)]
pub enum StakeCommand {
    #[command()]
    Withdraw(WithdrawArgs),
}

#[derive(Debug, Args, Default)]
pub struct WithdrawArgs {
    pub account: String,
}
