use std::io::Write;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use console::Term;
use copypasta::{ClipboardContext, ClipboardProvider};
use monedero_solana::monedero_mesh::{
    auth_token, Dapp, KvStorage, Metadata, NoopSessionHandler, Pairing, ProjectId, ProposeFuture,
    WalletConnectBuilder,
};
use monedero_solana::{
    ReownSigner, SolanaSession, SolanaWallet, TokenAccountsClient, TokenMetadataClient,
    TokenTransferClient,
};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;

use crate::cli::{Cli, SubCommands};
use crate::cmd::MainMenu;
use crate::config::AppConfig;
use crate::context::Context;
use crate::log::initialize_logging;

mod cli;
mod cmd;
mod config;
mod context;
mod log;

async fn init_dapp(cfg: AppConfig) -> anyhow::Result<(Pairing, ProposeFuture, bool)> {
    let project = ProjectId::from("5c9d8a326d3afb25ed1dff90f6d1807a");
    let auth = auth_token("https://github.com/dougEfresh");
    let storage_path = format!("{}", cfg.storage()?.display());
    let storage = KvStorage::file(Some(storage_path))?;
    let mgr = WalletConnectBuilder::new(project, auth)
        .store(storage)
        .build()
        .await?;
    let dapp = Dapp::new(
        mgr,
        Metadata {
            name: env!("CARGO_BIN_NAME").to_string(),
            description: "monedero mesh cli dapp".to_string(),
            url: "https://github.com/dougeEfresh/monedero-mesh".to_string(),
            icons: vec![],
            verify_url: None,
            redirect: None,
        },
    )
    .await?;

    let (p, fut, cached) = dapp.propose(NoopSessionHandler, &cfg.chains()).await?;
    Ok((p, fut, cached))
}

async fn show_pair(pairing: Pairing) {
    println!("Pairing: {:?}", pairing);
}

async fn main_menu(mut ctx: Context) -> anyhow::Result<()> {
    let menu_items = vec![
        MainMenu::Transfer,
        MainMenu::Tokens,
        MainMenu::Stake,
        MainMenu::Swap,
        MainMenu::Quit,
    ];
    loop {
        let mut p = promkit::preset::listbox::Listbox::new(&menu_items)
            .title("Main Menu")
            .prompt()?;
        let item = MainMenu::from_str(&p.run()?).expect("blah");
        let result = match item {
            MainMenu::Transfer => cmd::transfer::invoke(&mut ctx).await,
            MainMenu::Tokens => cmd::tokens::invoke(&mut ctx).await,
            MainMenu::Stake => cmd::stake::invoke(&mut ctx).await,
            MainMenu::Quit => break,
            _ => Ok(()),
        };
        if let Err(e) = result {
            tracing::error!("error! {}", e);
            writeln!(ctx.term, "error: {}", e)?;
            let mut p = promkit::preset::confirm::Confirm::new("Continue?").prompt()?;
            let confirm = p.run()?;
            if confirm.contains("n") {
                break;
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let matches = cli::Cli::parse();
    initialize_logging()?;
    let mut ctx = ClipboardContext::new().expect("Failed to open clipboard");

    let cfg = AppConfig::new(matches.config.clone(), matches.profile.clone())?;
    let (pairing, fut, cached) = init_dapp(cfg.clone()).await?;
    let mut term = Term::stdout();
    if !cached {
        ctx.set_contents(pairing.to_string())
            .expect("Failed to set clipboard");
        writeln!(term, "Pairing: {}", pairing)?;
    }

    tracing::info!("restored from cache {:?}", cached);
    let cs = fut.await?;
    if let Err(e) = cs.ping().await {
        tracing::info!("ping error {e}");
    }

    let sol_session = SolanaSession::try_from(&cs)?;
    let rpc_client = cfg.solana_rpc_client();
    let storage_path = cfg.storage()?;
    let wallet = SolanaWallet::init(
        sol_session.clone(),
        rpc_client,
        storage_path,
        matches.max_fee,
        None,
    )
    .await?;

    let ctx = Context { wallet, term };
    match matches.subcommands {
        None => {
            /*
            let helius = Pubkey::from_str("he1iusunGwqrNtafDtLdhsUQDFvo13z9sUa36PauBtk")?;
            let staker = ctx.wallet.stake_client();
            let (_, sig) = staker
                .create_delegate(5 * LAMPORTS_PER_SOL, &helius)
                .await?;
            ctx.term.write_line(&format!("{sig}"))?;
            Ok(())
            */
            ctx.term.clear_screen()?;
            tokio::spawn(async move { cs.pinger(Duration::from_secs(15)).await });
            main_menu(ctx).await
        }
        Some(SubCommands::Fees) => {
            let fees = ctx.wallet.fees().await?;
            ctx.term.write_line(&format!("{:?}", fees))?;
            Ok(())
        }
        Some(SubCommands::Balance) => {
            let balance = ctx.wallet.balance().await? as f64 / LAMPORTS_PER_SOL as f64;
            ctx.term.write_line(&format!("{balance}"))?;
            Ok(())
        }
        _ => Ok(()),
    }
}
