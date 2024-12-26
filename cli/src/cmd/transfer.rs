use clap::{Args, Subcommand};

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

    pub to: String,
    pub sol: f64,
}
