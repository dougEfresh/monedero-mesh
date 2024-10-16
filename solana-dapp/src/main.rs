use std::collections::HashMap;
use std::sync::Arc;
use std::thread::spawn;
use std::time::Duration;
use copypasta::{ClipboardContext, ClipboardProvider};
use ratatui::widgets::{Block, Paragraph, Tabs};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::mpsc::error::TryRecvError;
use crate::log::initialize_logging;

//pub mod app;
mod config;
// pub mod handler;
mod log;
mod state;
mod pubsub;
mod pair;
mod account;
// mod message;
// mod session_poll;
// pub mod ui;

use crate::config::AppConfig;
use monedero_solana::monedero_mesh::{auth_token, ClientSession, Dapp, Metadata, NoopSessionHandler, Pairing, ProjectId, ProposeFuture, WalletConnectBuilder};

async fn init_dapp(cfg: AppConfig) -> anyhow::Result<(Dapp, Pairing, ProposeFuture, bool)> {
    let project = ProjectId::from("1760736b8b49aeb707b1a80099e51e58");
    let auth = auth_token("https://github.com/dougEfresh");
    let mgr = WalletConnectBuilder::new(project, auth).build().await?;
    let dapp = Dapp::new(
        mgr,
        Metadata {
            name: env!("CARGO_CRATE_NAME").to_string(),
            description: "solana dapp tui with walletconnect".to_string(),
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

use widgetui::*;
use widgetui::ratatui::prelude::*;
use widgetui::widget::WidgetError;
use monedero_namespaces::{Accounts, ChainId};
use monedero_solana::{KeyedStakeState, SolanaSession, StakeClient, WalletConnectSigner};
use pair::PairingState;
use crate::account::AccountType;
use crate::pubsub::PubSub;
use crate::state::{AccountRx, AccountState, AccountTx};

#[derive(Clone, PartialEq, Debug)]
enum SettlementState {
    Error(String),
    Settled,
}

pub struct CustomChunk;
pub struct HeaderChunk;
pub struct AnotherChunk;
pub struct HomeChunk;

pub fn chunk_generator(frame: Res<WidgetFrame>, mut chunks: ResMut<Chunks>) -> WidgetResult {

    let v = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(15),
                Constraint::Percentage(85),
            ]
                .as_ref(),
        )
        .split(frame.size());

    chunks.register_chunk::<HeaderChunk>(v[0]);
    chunks.register_chunk::<HomeChunk>(v[1]);

    Ok(())
}


fn app_widget(
    mut frame: ResMut<WidgetFrame>,
    state: Res<SessionState>,
    mut account_state: ResMut<AccountState>,
    mut events: ResMut<Events>,
    mut chunks: Res<Chunks>) -> WidgetResult {

    let home = chunks.get_chunk::<HomeChunk>()?;
    let header = chunks.get_chunk::<HeaderChunk>()?;

    if events.key(crossterm::event::KeyCode::Char('q')) {
        events.register_exit();
        return Ok(())
    }

    let head = Tabs::new(vec!["Tab1", "Tab2", "Tab3", "Tab4"])
        .block(Block::bordered().title("Tabs"))
        .style(Style::default().white())
        .highlight_style(Style::default().yellow())
        .select(2)
        .divider(symbols::DOT)
        .padding("->", "<-");
    frame.render_widget(
        head,
        header,
    );
    frame.render_widget(
        Paragraph::new(format!("{} balance:{}", state.sol_session.pubkey(), account_state.updated_balance() as f64 / LAMPORTS_PER_SOL as f64 )),
        home,
    );
    Ok(())
}


struct SessionState {
    dapp: Dapp,
    rpc_client: Arc<RpcClient>,
    sol_session: SolanaSession,
    signer: WalletConnectSigner,
}


impl State for SessionState {

}

async fn pair_ping(dapp: Dapp) {
    loop {
        if let Err(e) = dapp.pair_ping().await {
            tracing::warn!("pair ping failed! {e}");
        }
        tokio::time::sleep(Duration::from_secs(20)).await;
    }
}


async fn show_pair(dapp: Dapp, pairing: Pairing, rx: tokio::sync::mpsc::UnboundedReceiver<SettlementState>) -> anyhow::Result<()> {
    let state = PairingState{ pairing, rx, settlement: None };
    let mut app = App::new(100)?;
    if let Err(err) =  app.states(state).
        widgets(pair::pair_widget).handle_panics().run() {
        if err.to_string().starts_with("quit") {
            let _ = dapp.purge().await;
            return Ok(())
        }
        return anyhow::bail!(err);
    }
    Ok(())
}

async fn subscriptions(url: &str, accounts: Vec<AccountType>, account_tx: AccountTx, tx: tokio::sync::broadcast::Sender<bool>) -> anyhow::Result<()> {
    let ps = PubSub::new(url, account_tx).await?;
    let ps_cloned = ps.clone();
    let mut terminator = tx.subscribe();
    tokio::spawn(async move  {
        ps_cloned.slots(terminator).await
    });
    for acct in accounts {
        let ps_cloned = ps.clone();
        let mut terminator = tx.subscribe();
        tokio::spawn(async move {
            ps_cloned.run(acct,  terminator).await
        });
    }
    Ok(())
}

async fn app() -> anyhow::Result<()> {
    let cfg = AppConfig::default();
    let (dapp, pairing, fut, cached) = init_dapp(cfg.clone()).await?;
    let mut ctx = ClipboardContext::new().expect("Failed to open clipboard");
    ctx.set_contents(pairing.to_string()).expect("Failed to set clipboard");
    tracing::info!("restored from cache {:?}", cached);


    dotenvy::dotenv()?;
    let custom_rpc = std::env::var("solana_8E9rvCKLFQia2Y35HXjjpWzj8weVo44K").ok();
    let rpc_url = if custom_rpc.is_some() { custom_rpc.unwrap() } else { cfg.solana_rpc.clone() };
    let ws_url = rpc_url.replace("https://", "wss://");
    tracing::info!("RPC URL {} WS_URL {}", rpc_url, ws_url);
    let rpc_client = Arc::new(RpcClient::new(rpc_url));
    let stats = rpc_client.clone();
    let sol_session = SolanaSession::try_from(&cs)?;
    let pk = sol_session.pubkey();
    let signer = WalletConnectSigner::new(sol_session.clone());
    let staker = StakeClient::new(sol_session.clone(), signer.clone(), rpc_client.clone());
    let stake_accounts = staker.accounts().await?;

    tokio::spawn(pair_ping(dapp.clone()));
    let state = SessionState{
        dapp,
        rpc_client,
        sol_session,
        signer,
    };

    let (balance_tx, balance_rx) = tokio::sync::watch::channel::<u64>(0);
    let balance = stats.get_balance(&pk).await.ok().unwrap_or_default();
    let mut accounts: Vec<AccountType> = stake_accounts.iter().map(|s| AccountType::Stake(s.stake_pubkey)).collect();
    accounts.push(AccountType::Native(pk));
    let account_rx = AccountRx {
        balance_rx
    };
    let account_tx = AccountTx {
        balance_tx,
    };
    let account_state = AccountState::new(balance, stake_accounts, account_rx);
    let (terminate_tx, _) = tokio::sync::broadcast::channel::<bool>(1);
    subscriptions(&ws_url, accounts, account_tx, terminate_tx.clone()).await?;
    let mut app = App::new(100)?;
    app.states(state).states(account_state).widgets((chunk_generator, app_widget)).handle_panics().run()?;
    let s = stats.get_transport_stats();
    tracing::info!("transport stats {}", s.request_count);
    terminate_tx.send(true).expect("Failed to send terminate signal");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_logging()?;
    app().await?;
    Ok(())
}
