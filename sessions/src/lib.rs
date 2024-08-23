mod actors;
pub mod crypto;
mod dapp;
mod domain;
mod error;
mod handlers;
mod pair;
pub mod pairing_uri;
mod relay;
pub mod rpc;
pub mod session;
mod storage;
mod transport;
// mod wallet;

use crate::rpc::SessionSettleRequest;
pub use crypto::cipher::Cipher;
pub use domain::Message;
pub use error::Error;
pub use pair::{PairingManager, WalletConnectBuilder};
pub use pairing_uri::Pairing;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::sync::{Arc, Mutex, Once, OnceLock};
pub use storage::KvStorage;
use tokio::sync::broadcast;
pub use walletconnect_sdk::client::error::ClientError;

use crate::domain::MessageId;
use crate::relay::ConnectionHandler;
use crate::session::ClientSession;
pub use transport::WireEvent;

pub type EventChannel = broadcast::Receiver<WireEvent>;
pub type EventClientSession = tokio::sync::oneshot::Receiver<Result<ClientSession>>;
pub type Atomic<T> = Arc<Mutex<T>>;

#[derive(Debug, Default, Clone, PartialEq)]
pub enum SocketEvent {
    Connected,
    #[default]
    Disconnect,
    ForceDisconnect,
}

impl Display for SocketEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SocketEvent::Connected => {
                write!(f, "connected")
            }
            SocketEvent::Disconnect => {
                write!(f, "disconnected")
            }
            SocketEvent::ForceDisconnect => {
                write!(f, "force disconnect")
            }
        }
    }
}

//#[trait_variant::make(Send)]
pub trait SocketHandler {
    fn event(&self, event: SocketEvent);
}

pub type Result<T> = std::result::Result<T, Error>;
#[allow(dead_code)]
static INIT: Once = Once::new();

pub const RELAY_ADDRESS: &str = "wss://relay.walletconnect.com";

pub(crate) struct Settlement(pub MessageId, pub SessionSettleRequest);

pub(crate) fn send_event(tx: &broadcast::Sender<WireEvent>, event: WireEvent) {
    if let Err(err) = tx.send(event.clone()) {
        tracing::error!("Failed to broadcast event {err} {event:#?}");
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::actors::{Actors, RegisteredManagers};
    use crate::domain::ProjectId;
    use crate::relay::mock::test::auth;
    use crate::{Cipher, Pairing, PairingManager, WalletConnectBuilder, INIT};
    use std::str::FromStr;
    use std::time::Duration;
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::EnvFilter;

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
        let dapp = WalletConnectBuilder::new(p.clone(), auth()).build().await?;
        let wallet = WalletConnectBuilder::new(p, auth()).build().await?;
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
        if (pair) {
            dapp_wallet_ciphers(&t).await?;
            let registered = wallet_actors.request().send(RegisteredManagers).await?;
            assert_eq!(1, registered);
            let registered = dapp_actors.request().send(RegisteredManagers).await?;
            assert_eq!(1, registered);
        }
        Ok(t)
    }

    pub(crate) async fn dapp_wallet_ciphers(t: &TestStuff) -> anyhow::Result<()> {
        let pairing = Pairing::default();
        let topic = pairing.topic.clone();

        t.dapp_cipher.set_pairing(Some(pairing.clone()))?;
        let pairing_from_uri = Pairing::from_str(&t.dapp_cipher.pairing_uri().unwrap())?;
        t.wallet_cipher.set_pairing(Some(pairing_from_uri))?;

        t.dapp_cipher
            .create_common_topic(t.wallet_cipher.public_key_hex().unwrap())?;
        let _ = t.wallet_cipher.create_common_topic(
            t.dapp_cipher
                .public_key_hex()
                .ok_or(crate::Error::NoPairingTopic)?,
        );

        t.dapp_actors
            .register_mgr(topic.clone(), t.dapp.clone())
            .await?;
        t.wallet_actors
            .register_mgr(topic.clone(), t.wallet.clone())
            .await?;

        t.dapp.subscribe(topic.clone()).await?;
        t.wallet.subscribe(topic.clone()).await?;
        yield_ms(1000);

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
}
