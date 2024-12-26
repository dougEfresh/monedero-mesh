use {
    clap::{Args, Subcommand},
    solana_pubkey::Pubkey,
    std::str::FromStr,
};

#[derive(Debug, Args)]
pub struct TransferArgs {
    #[command(subcommand)]
    pub command: TransferCommand,
}

#[derive(Debug, Subcommand)]
pub enum TransferCommand {
    #[command()]
    Native(SendArgs),
}

#[derive(Debug, Args, Default)]
pub struct SendArgs {
    #[arg(help = "fund receipiant account", long)]
    pub fund: bool,

    #[arg(help = "send to this pubkey")]
    pub to: Pubkey,
    pub sol: f64,
}
