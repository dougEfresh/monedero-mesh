use crate::cmd::MainMenu;
use crate::config::AppConfig;
use crate::context::Context;
use crate::log::initialize_logging;
use console::Term;
use copypasta::{ClipboardContext, ClipboardProvider};
use dialoguer::theme::ColorfulTheme;
use monedero_solana::monedero_mesh::{
    auth_token, Dapp, KvStorage, Metadata, NoopSessionHandler, Pairing, ProjectId, ProposeFuture,
    WalletConnectBuilder,
};
use monedero_solana::{
    ReownSigner, SolanaSession, SolanaWallet, TokenAccountsClient, TokenMetadataClient,
    TokenTransferClient,
};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use std::io::Write;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

mod cmd;
mod config;
mod context;
mod log;

async fn pair_ping(dapp: Dapp) {
    loop {
        if let Err(e) = dapp.pair_ping().await {
            tracing::warn!("pair ping failed! {e}");
        }
        tokio::time::sleep(Duration::from_secs(20)).await;
    }
}

async fn init_dapp(cfg: AppConfig) -> anyhow::Result<(Dapp, Pairing, ProposeFuture, bool)> {
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
            name: env!("CARGO_CRATE_NAME").to_string(),
            description: "monedero mesh cli dapp".to_string(),
            url: "https://github.com/dougeEfresh/monedero-mesh".to_string(),
            icons: vec![],
            verify_url: None,
            redirect: None,
        },
    )
    .await?;

    let (p, fut, cached) = dapp.propose(NoopSessionHandler, &cfg.chains()).await?;
    Ok((dapp, p, fut, cached))
}

async fn show_pair(pairing: Pairing) {
    println!("Pairing: {:?}", pairing);
}

async fn main_menu(mut ctx: Context) -> anyhow::Result<()> {
    let menu_items = vec![
        cmd::MainMenu::Transfer,
        cmd::MainMenu::Tokens,
        cmd::MainMenu::Stake,
        cmd::MainMenu::Swap,
        cmd::MainMenu::Quit,
    ];
    loop {
        let mut p = promkit::preset::listbox::Listbox::new(&menu_items)
            .title("Main Menu")
            .prompt()?;
        let item = MainMenu::from_str(&p.run()?).expect("blah");
        let result = match item {
            MainMenu::Transfer => cmd::transfer::invoke(&mut ctx).await,
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
    /*
    loop {
        let menu_item = dialoguer::Select::with_theme(&ColorfulTheme::default()).items(&menu_items).interact_on(&ctx.term)?;
        let result = match &menu_items[menu_item] {
            MainMenu::Transfer => {
                cmd::transfer::invoke(&mut ctx).await
            }
            MainMenu::Quit => break,
            _ => Ok(())
        };
        if let Err(e) = result {
            tracing::error!("error! {}", e);
            writeln!(ctx.term, "error: {}", e)?;
            let confirm = dialoguer::Confirm::with_theme(&ColorfulTheme::default()).with_prompt("Continue?").interact_on(&ctx.term)?;
            if !confirm {
                break
            }
        }
        ctx.term.clear_screen()?;
    }
     */
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_logging()?;
    dotenvy::dotenv()?;
    let mut ctx = ClipboardContext::new().expect("Failed to open clipboard");
    let cfg = AppConfig::default();
    let (dapp, pairing, fut, cached) = init_dapp(cfg.clone()).await?;
    ctx.set_contents(pairing.to_string())
        .expect("Failed to set clipboard");
    let mut term = Term::stdout();
    if !cached {
        writeln!(term, "Pairing: {}", pairing)?;
    }

    tracing::info!("restored from cache {:?}", cached);
    let cs = fut.await?;
    if let Err(e) = cs.ping().await {
        tracing::info!("ping error {e}");
    }

    let sol_session = SolanaSession::try_from(&cs)?;
    let signer = ReownSigner::new(sol_session.clone());
    let rpc_client = cfg.rpc_client();
    tokio::spawn(pair_ping(dapp.clone()));
    let pk = sol_session.pubkey();
    term.clear_screen()?;
    //let signer = WalletConnectSigner::new(sol_session.clone());
    //write!(term, "Chain: {} Account: {} Balance: {}\n", sol_session.chain_type(), pk, sol_session.balance(&rpc_client).await )?;
    let storage_path = cfg.storage()?;
    let wallet = SolanaWallet::init(sol_session.clone(), rpc_client, storage_path).await?;
    //let mut hist = dialoguer::BasicHistory::new().max_entries(8).no_duplicates(true);
    let mut ctx = Context {
        sol_session,
        wallet,
        term,
    };
    main_menu(ctx).await?;
    Ok(())
}
