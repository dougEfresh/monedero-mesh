pub mod crypto;
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
mod actors;
// mod wallet;

use std::fmt::{Display, Formatter};
use crate::rpc::SessionSettleRequest;
pub use crypto::cipher::Cipher;
pub use domain::Message;
pub use error::Error;
pub use pair::{PairingManager, WalletConnectBuilder};
pub use pairing_uri::Pairing;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, Once, OnceLock};
pub use storage::KvStorage;
use tokio::sync::broadcast;
pub use walletconnect_sdk::client::error::ClientError;

use crate::domain::MessageId;
use crate::relay::ConnectionHandler;
pub use transport::WireEvent;
use crate::session::ClientSession;

pub type EventChannel = broadcast::Receiver<WireEvent>;
pub type EventClientSession = tokio::sync::oneshot::Receiver<Result<ClientSession>>;
pub type Atomic<T> = Arc<Mutex<T>>;

#[derive(Debug, Default, Clone, PartialEq)]
pub enum SocketEvent {
    Connected,
    #[default]
    Disconnect,
    ForceDisconnect
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
pub trait SocketHandler: Sync + Send + 'static {
    fn event(&mut self, event: SocketEvent);
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
    use std::cell::RefCell;
    use std::str::FromStr;
    use std::sync::{Arc, RwLock};
    use std::time::Duration;
    use crate::{Cipher, KvStorage, Pairing, PairingManager, SocketEvent, SocketHandler, INIT};
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::EnvFilter;
    use crate::actors::Actors;

    #[derive(Default, Debug, Clone)]
    pub (crate) struct  ConnectionState {
        pub(crate) state: Arc<RwLock<SocketEvent>>
    }

    impl SocketHandler for ConnectionState {
        fn event(&mut self, event: SocketEvent) {
            if let Ok(mut l) =  self.state.write() {
                *l = event;
            }
        }
    }

    pub(crate) struct TestStuff {
        pub(crate) dapp_cipher: Cipher,
        pub(crate) wallet_cipher: Cipher,
        pub(crate) actors: Actors,
        pub(crate) socket_state: ConnectionState,
        //dapp: PairingManager,
        //wallet: PairingManager,
    }

    pub(crate) async fn yield_ms(ms: u64) {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }

    pub(crate) async fn init_test_components() -> anyhow::Result<TestStuff> {
        let (dapp_cipher, wallet_cipher) = dapp_wallet_ciphers()?;
        let actors = Actors::init(dapp_cipher.clone());
        let socket_state = ConnectionState::default();
        actors.register_socket_handler(socket_state.clone()).await?;
        Ok(TestStuff{
            dapp_cipher,
            wallet_cipher,
            actors,
            socket_state,
        })
    }
    pub(crate) fn dapp_wallet_ciphers() -> anyhow::Result<(Cipher, Cipher)> {
        init_tracing();
        let dapp = Cipher::new(Arc::new(KvStorage::default()), None)?;
        let wallet = Cipher::new(Arc::new(KvStorage::default()), None)?;
        let pairing = Pairing::default();
        let topic = pairing.topic.clone();

        dapp.set_pairing(Some(pairing.clone()))?;
        let pairing_from_uri = Pairing::from_str(&dapp.pairing_uri().unwrap())?;
        wallet.set_pairing(Some(pairing_from_uri))?;

        dapp.create_common_topic(wallet.public_key_hex().unwrap())?;
        wallet.create_common_topic(dapp.public_key_hex().unwrap())?;
        Ok((dapp, wallet))
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
