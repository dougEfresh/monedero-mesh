use assert_matches::assert_matches;
use std::collections::BTreeMap;
use std::sync::Once;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;
use walletconnect_namespaces::{ChainId, ChainType, NamespaceName, Namespaces};
use walletconnect_relay::{auth_token, ConnectionCategory, ConnectionOptions, ConnectionPair};
use walletconnect_sessions::crypto::CipherError;
use walletconnect_sessions::rpc::Metadata;
use walletconnect_sessions::{
    Actors, ClientSession, Dapp, NoopSessionHandler, ProjectId, ProposeFuture, Wallet,
    WalletConnectBuilder,
};
use walletconnect_sessions::{Result, Topic};

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
    let shared_id = Topic::generate();
    let p = ProjectId::from("9d5b20b16777cc49100cf9df3649bd24");
    let auth = auth_token("https://github.com/dougEfresh");
    let dapp_id = ConnectionPair(shared_id.clone(), ConnectionCategory::Dapp);
    let wallet_id = ConnectionPair(shared_id.clone(), ConnectionCategory::Wallet);
    let dapp_opts = ConnectionOptions::new(p.clone(), auth.clone(), dapp_id);
    let wallet_opts = ConnectionOptions::new(p, auth, wallet_id);
    let dapp_manager =
        WalletConnectBuilder::new(p.clone(), auth_token("https://github.com/dougEfresh"))
            .connect_opts(dapp_opts)
            .build()
            .await?;
    let wallet_manager = WalletConnectBuilder::new(p, auth_token("https://github.com/dougEfresh"))
        .connect_opts(wallet_opts)
        .build()
        .await?;
    let dapp_actors = dapp_manager.actors();
    let wallet_actors = wallet_manager.actors();
    let dapp = Dapp::new(dapp_manager, Metadata::default());
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
    let (pairing, rx) = dapp
        .propose(
            NoopSessionHandler,
            &[
                ChainId::EIP155(alloy_chains::Chain::holesky()),
                ChainId::Solana(ChainType::Test),
            ],
        )
        .await?;
    info!("got pairing topic {pairing}");
    let (_, wallet_rx) = wallet.pair(pairing.to_string(), NoopSessionHandler).await?;
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
    assert!(session.namespaces.contains_key(&NamespaceName::Solana));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_dapp_ping() -> anyhow::Result<()> {
    let session = pair_dapp_wallet().await?;
    assert!(session.ping().await?);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_dapp_delete() -> anyhow::Result<()> {
    let session = pair_dapp_wallet().await?;
    assert!(session.delete().await?);
    assert_matches!(
        session.ping().await,
        Err(walletconnect_sessions::Error::CipherError(
            CipherError::UnknownTopic(_)
        ))
    );
    Ok(())
}
