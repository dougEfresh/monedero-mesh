use anyhow::format_err;
use assert_matches::assert_matches;
use std::collections::BTreeMap;
use std::sync::Once;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;
use walletconnect_namespaces::{AlloyChain, ChainId, ChainType, NamespaceName, Namespaces};
use walletconnect_relay::{auth_token, ConnectionCategory, ConnectionOptions, ConnectionPair};
use walletconnect_sessions::crypto::CipherError;
use walletconnect_sessions::rpc::Metadata;
use walletconnect_sessions::{
    Actors, ClientSession, Dapp, NoopSessionHandler, ProjectId, ProposeFuture,
    RegisteredComponents, Wallet, WalletConnectBuilder,
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
    let wallet_opts = ConnectionOptions::new(p.clone(), auth.clone(), wallet_id);
    let dapp_manager = WalletConnectBuilder::new(p.clone(), auth.clone())
        .connect_opts(dapp_opts)
        .build()
        .await?;
    let wallet_manager = WalletConnectBuilder::new(p, auth)
        .connect_opts(wallet_opts)
        .build()
        .await?;
    let dapp_actors = dapp_manager.actors();
    let wallet_actors = wallet_manager.actors();
    let md = Metadata {
        name: "mock-dapp".to_string(),
        ..Default::default()
    };
    let dapp = Dapp::new(dapp_manager, md).await?;
    let wallet = Wallet::new(wallet_manager).await?;
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

async fn await_wallet_pair(rx: ProposeFuture) {
    match timeout(Duration::from_secs(5), rx).await {
        Ok(s) => match s {
            Ok(_) => {
                info!("wallet got client session")
            }
            Err(e) => error!("wallet got error session: {e}"),
        },
        Err(e) => error!("timeout for wallet to recv client session from wallet: {e}"),
    }
}

async fn pair_dapp_wallet() -> anyhow::Result<(TestStuff, ClientSession)> {
    let t = init_test_components().await?;
    let dapp = t.dapp.clone();
    let wallet = t.wallet.clone();
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
    let session = timeout(Duration::from_secs(5), rx).await??;
    // let wallet get their ClientSession
    yield_ms(1000).await;
    Ok((t, session))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_dapp_settlement() -> anyhow::Result<()> {
    let (test, session) = pair_dapp_wallet().await?;
    info!("settlement complete");
    assert!(session.namespaces().contains_key(&NamespaceName::Solana));
    assert!(session.ping().await?);
    assert!(session.delete().await);
    let components = test
        .dapp_actors
        .session()
        .send(RegisteredComponents)
        .await?;
    assert_eq!(0, components);
    assert_matches!(
        session.ping().await,
        Err(walletconnect_sessions::Error::NoClientSession(_))
    );

    // propose again should not re-pair

    let original_pairing = test.dapp.pairing().ok_or(format_err!("no pairing!"))?;
    let (new_pairing, rx) = test
        .dapp
        .propose(
            NoopSessionHandler,
            &[ChainId::EIP155(AlloyChain::sepolia())],
        )
        .await?;
    assert_eq!(original_pairing.topic, new_pairing.topic);
    let (wallet_pairing, wallet_rx) = test
        .wallet
        .pair(original_pairing.to_string(), NoopSessionHandler)
        .await?;
    assert_eq!(wallet_pairing.topic, new_pairing.topic);
    let session = timeout(Duration::from_secs(5), rx).await??;
    yield_ms(5000).await;
    Ok(())
}
