use assert_matches::assert_matches;
use std::collections::BTreeMap;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Once;
use std::time::Duration;
use tokio::time::timeout;
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;
use walletconnect_sessions::rpc::{ProposeNamespace, ProposeNamespaces};
use walletconnect_sessions::{
    auth_token, Actors, Dapp, ProjectId, Topic, Wallet, WalletConnectBuilder,
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

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_dapp_settlement() -> anyhow::Result<()> {
    let t = init_test_components().await?;
    let dapp = t.dapp;
    let wallet = t.wallet;
    let (pairing, rx) = dapp.propose(namespaces()).await?;
    info!("got pairing topic {pairing}");
    wallet.pair(pairing.to_string()).await?;
    info!("wallet has been paired");
    let session = timeout(Duration::from_secs(5), rx).await???;
    info!("settlement complete");
    yield_ms(1000).await;
    assert!(session.namespaces.deref().contains_key("eip155"));
    Ok(())
}
