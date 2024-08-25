use assert_matches::assert_matches;
use std::str::FromStr;
use std::sync::Once;
use std::time::Duration;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;
use walletconnect_sessions::ProjectId;
use walletconnect_sessions::{auth_token, Actors, Topic};
use walletconnect_sessions::{Cipher, Pairing, PairingManager, WalletConnectBuilder};

#[allow(dead_code)]
static INIT: Once = Once::new();

pub(crate) struct TestStuff {
    pub(crate) dapp_cipher: Cipher,
    pub(crate) wallet_cipher: Cipher,
    pub(crate) dapp_actors: Actors,
    pub(crate) wallet_actors: Actors,
    pub(crate) dapp: PairingManager,
    pub(crate) wallet: PairingManager,
}

pub(crate) async fn yield_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

pub(crate) async fn init_test_components(pair: bool) -> anyhow::Result<TestStuff> {
    init_tracing();
    let p = ProjectId::from("9d5b20b16777cc49100cf9df3649bd24");
    let dapp =
        WalletConnectBuilder::new(p.clone(), auth_token(String::from("https://example.com")))
            .build()
            .await?;
    let wallet =
        WalletConnectBuilder::new(p, auth_token(String::from("https://github.com/dougEfresh")))
            .build()
            .await?;
    let dapp_actors = dapp.actors();
    let wallet_actors = wallet.actors();
    yield_ms(500).await;
    let t = TestStuff {
        dapp_cipher: dapp.ciphers(),
        wallet_cipher: wallet.ciphers(),
        dapp_actors: dapp_actors.clone(),
        wallet_actors: wallet_actors.clone(),
        dapp,
        wallet,
    };
    if pair {
        dapp_wallet_ciphers(&t).await?;
        let registered = wallet_actors.registered_managers().await?;
        assert_eq!(1, registered);
        let registered = dapp_actors.registered_managers().await?;
        assert_eq!(1, registered);
    }
    Ok(t)
}

pub(crate) async fn dapp_wallet_ciphers(t: &TestStuff) -> anyhow::Result<()> {
    let pairing = Pairing::default();
    t.dapp.set_pairing(pairing.clone()).await?;
    let pairing_from_uri = Pairing::from_str(&t.dapp_cipher.pairing_uri().unwrap())?;
    t.wallet.set_pairing(pairing_from_uri).await?;

    t.dapp_cipher
        .create_common_topic(t.wallet_cipher.public_key_hex().unwrap())?;
    let _ = t.wallet_cipher.create_common_topic(
        t.dapp_cipher
            .public_key_hex()
            .ok_or(walletconnect_sessions::Error::NoPairingTopic)?,
    );

    yield_ms(1000).await;
    Ok(())
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

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_relay_pair_ping() -> anyhow::Result<()> {
    let test_components = init_test_components(true).await?;
    let dapp = test_components.dapp;
    dapp.ping().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_relay_pair_delete() -> anyhow::Result<()> {
    let test_components = init_test_components(true).await?;
    let dapp = test_components.dapp;
    dapp.delete().await?;
    let c = dapp.ciphers();
    yield_ms(2000).await;
    assert!(c.pairing().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_relay_pair_extend() -> anyhow::Result<()> {
    let test_components = init_test_components(true).await?;
    let dapp = test_components.dapp;
    dapp.extend(100000).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_relay_disconnect() -> anyhow::Result<()> {
    let test_components = init_test_components(true).await?;
    let dapp = test_components.dapp;
    // special topic to indicate a force disconnect
    let disconnect_topic =
        Topic::from("92b2701dbdbb72abea51591a06d41e7d76ebfe18e1a1ca5680a5ac6e3717c6d9");
    dapp.subscribe(disconnect_topic.clone()).await?;
    yield_ms(555).await;
    assert_matches!(
        dapp.ping().await,
        Err(walletconnect_sessions::Error::ConnectError(
            walletconnect_sessions::ClientError::Disconnected
        ))
    );
    yield_ms(3300).await;
    // should have reconnected
    dapp.ping().await?;
    Ok(())
}
