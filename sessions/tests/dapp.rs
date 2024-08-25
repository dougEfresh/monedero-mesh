use std::collections::BTreeMap;
use std::sync::Once;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;
use walletconnect_sessions::rpc::{ProposeFuture, ProposeNamespace, ProposeNamespaces};
use walletconnect_sessions::Result;
use walletconnect_sessions::{
    auth_token, Actors, ClientSession, Dapp, ProjectId, Wallet, WalletConnectBuilder,
};

#[allow(dead_code)]
static INIT: Once = Once::new();

pub(crate) struct TestStuff {
    pub(crate) dapp_actors: Actors,
    pub(crate) wallet_actors: Actors,
    pub(crate) dapp: Dapp,
    pub(crate) wallet: Wallet,
}

pub(crate) async fn yield_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

pub(crate) async fn init_test_components() -> anyhow::Result<TestStuff> {
    init_tracing();
    let p = ProjectId::from("9d5b20b16777cc49100cf9df3649bd24");
    let dapp_manager =
        WalletConnectBuilder::new(p.clone(), auth_token("https://github.com/dougEfresh"))
            .build()
            .await?;
    let wallet_manager = WalletConnectBuilder::new(p, auth_token("https://github.com/dougEfresh"))
        .build()
        .await?;
    let dapp_actors = dapp_manager.actors();
    let wallet_actors = wallet_manager.actors();
    let dapp = Dapp::new(dapp_manager);
    let wallet = Wallet::new(wallet_manager);
    yield_ms(500).await;
    let t = TestStuff {
        dapp_actors: dapp_actors.clone(),
        wallet_actors: wallet_actors.clone(),
        dapp,
        wallet,
    };
    Ok(t)
}

pub(crate) fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_target(true)
            .with_level(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    });
}

fn namespaces() -> ProposeNamespaces {
    let mut namespaces: BTreeMap<String, ProposeNamespace> = BTreeMap::new();
    let required: Vec<alloy_chains::Chain> = vec![alloy_chains::Chain::sepolia()];
    namespaces.insert(String::from("eip155"), required.into());
    ProposeNamespaces(namespaces)
}

async fn await_wallet_pair(rx: ProposeFuture<Result<ClientSession>>) {
    match timeout(Duration::from_secs(5), rx).await {
        Ok(s) => match s {
            Ok(result) => match result {
                Err(e) => error!("wallet got client session error: {e}"),
                Ok(_) => info!("wallet got client session"),
            },
            Err(e) => error!("wallet got an recv channel client session: {e}"),
        },
        Err(e) => error!("timout for wallet to recv client session: {e}"),
    }
}

async fn pair_dapp_wallet() -> anyhow::Result<ClientSession> {
    let t = init_test_components().await?;
    let dapp = t.dapp;
    let wallet = t.wallet;
    let (pairing, rx) = dapp.propose(namespaces()).await?;
    info!("got pairing topic {pairing}");
    let (_, wallet_rx) = wallet.pair(pairing.to_string()).await?;
    tokio::spawn(async move { await_wallet_pair(wallet_rx).await });
    let session = timeout(Duration::from_secs(5), rx).await???;
    // let wallet get their ClientSession
    yield_ms(1000).await;
    Ok(session)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_dapp_settlement() -> anyhow::Result<()> {
    let session = pair_dapp_wallet().await?;
    info!("settlement complete");
    assert!(session.namespaces.contains_key("eip155"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_dapp_ping() -> anyhow::Result<()> {
    let session = pair_dapp_wallet().await?;
    assert!(session.ping().await?);
    Ok(())
}
