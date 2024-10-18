use std::str::FromStr;
use std::sync::{Arc, Mutex, Once};
use std::time::Duration;

use assert_matches::assert_matches;
use async_trait::async_trait;
use monedero_mesh::{
    Actors, Cipher, Pairing, PairingManager, ProjectId, RegisteredComponents, SocketEvent,
    SocketListener, Topic, WalletConnectBuilder,
};
use monedero_relay::{auth_token, ConnectionCategory, ConnectionOptions, ConnectionPair};
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

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

#[derive(Clone)]
struct DummySocketListener {
    pub events: Arc<Mutex<Vec<SocketEvent>>>,
}

impl DummySocketListener {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl SocketListener for DummySocketListener {
    async fn handle_socket_event(&self, event: SocketEvent) {
        let mut l = self.events.lock().unwrap();
        l.push(event);
    }
}

pub(crate) async fn yield_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

pub(crate) async fn init_test_components(pair: bool) -> anyhow::Result<TestStuff> {
    init_tracing();
    let shared_id = Topic::generate();
    let p = ProjectId::from("9d5b20b16777cc49100cf9df3649bd24");
    let auth = auth_token("https://github.com/dougEfresh");
    let dapp_id = ConnectionPair(shared_id.clone(), ConnectionCategory::Dapp);
    let wallet_id = ConnectionPair(shared_id.clone(), ConnectionCategory::Wallet);
    let dapp_opts = ConnectionOptions::new(p.clone(), auth.clone(), dapp_id);
    let wallet_opts = ConnectionOptions::new(p, auth, wallet_id);
    let p = ProjectId::from("9d5b20b16777cc49100cf9df3649bd24");
    let dapp =
        WalletConnectBuilder::new(p.clone(), auth_token(String::from("https://example.com")))
            .connect_opts(dapp_opts)
            .build()
            .await?;
    let wallet =
        WalletConnectBuilder::new(p, auth_token(String::from("https://github.com/dougEfresh")))
            .connect_opts(wallet_opts)
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
        let registered = wallet_actors.request().send(RegisteredComponents).await?;
        assert_eq!(1, registered);
        let registered = dapp_actors.request().send(RegisteredComponents).await?;
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
            .ok_or(monedero_mesh::Error::NoPairingTopic)?,
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
    let dapp_actors = test_components.dapp_actors;
    let components = dapp_actors.request().send(RegisteredComponents).await?;
    assert_eq!(0, components);
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
    let listener = DummySocketListener::new();
    dapp.register_socket_listener(listener.clone()).await;
    // special topic to indicate a force disconnect
    let disconnect_topic =
        Topic::from("92b2701dbdbb72abea51591a06d41e7d76ebfe18e1a1ca5680a5ac6e3717c6d9");
    dapp.subscribe(disconnect_topic.clone()).await?;
    yield_ms(1000).await;
    assert_matches!(
        dapp.ping().await,
        Err(monedero_mesh::Error::ConnectError(
            monedero_mesh::ClientError::Disconnected
        ))
    );
    info!("waiting for reconnect");
    yield_ms(3300).await;
    // should have reconnected
    dapp.ping().await?;
    let l = listener.events.lock()?;
    assert_eq!(2, l.len());
    Ok(())
}
