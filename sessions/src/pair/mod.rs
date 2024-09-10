mod builder;
mod handlers;
mod pairing;
mod registration;
mod socket_handler;

use crate::actors::Actors;
use crate::domain::{SubscriptionId, Topic};
use crate::relay::RelayHandler;
use crate::rpc::{
    ErrorParams, PairDeleteRequest, PairExtendRequest, PairPingRequest, RequestParams,
    SessionSettleRequest,
};
use crate::transport::TopicTransport;
use crate::{Cipher, Error, Pairing, Result, SessionSettled, SocketEvent, SocketListener};
pub use builder::WalletConnectBuilder;
use serde::de::DeserializeOwned;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn};
use walletconnect_namespaces::Namespaces;
use walletconnect_relay::{Client, ConnectionOptions};

#[derive(Clone, xtra::Actor)]
pub struct PairingManager {
    relay: Client,
    opts: ConnectionOptions,
    ciphers: Cipher,
    transport: TopicTransport,
    actors: Actors,
    pub(super) socket_listeners: Arc<tokio::sync::Mutex<Vec<Box<dyn SocketListener>>>>,
}

impl Debug for PairingManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let t: String = self
            .topic()
            .map(|t| crate::shorten_topic(&t))
            .unwrap_or(String::from("no-pairing"));
        write!(f, "pairing={} projectId={}", t, self.opts.project_id)
    }
}

impl PairingManager {
    async fn init(opts: ConnectionOptions, ciphers: Cipher) -> Result<Self> {
        let actors = Actors::init(ciphers.clone());
        let (socket_tx, socket_rx) = mpsc::unbounded_channel::<SocketEvent>();
        let handler = RelayHandler::new(
            ciphers.clone(),
            actors.request(),
            actors.response(),
            socket_tx,
        );
        #[cfg(feature = "mock")]
        let relay = Client::new(handler, &opts.conn_pair);

        #[cfg(not(feature = "mock"))]
        let relay = Client::new(handler);

        actors.register_client(relay.clone()).await?;
        relay.connect(&opts).await?;

        let transport = TopicTransport::new(actors.transport());

        let mgr = Self {
            relay,
            opts,
            ciphers,
            transport,
            actors: actors.clone(),
            socket_listeners: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        };
        actors.request().send(mgr.clone()).await?;
        let socket_handler = mgr.clone();
        tokio::spawn(socket_handler::handle_socket(socket_handler, socket_rx));
        mgr.open_socket().await?;
        mgr.restore_saved_pairing().await?;
        Ok(mgr)
    }

    pub async fn register_socket_listener<T: SocketListener>(&self, listener: T) {
        let mut l = self.socket_listeners.lock().await;
        l.push(Box::new(listener));
    }

    pub(crate) async fn resubsribe(&self) -> Result<()> {
        self.pairing().ok_or(Error::NoPairingTopic)?;
        let topics = self.ciphers.subscriptions();
        self.relay.batch_subscribe(topics).await?;
        Ok(())
    }

    /// Check if other side is "alive"i
    /// If the peer returns an RPC error then it is "alive"
    /// Error only for network communication errors or relay server is down
    pub(crate) async fn alive(&self) -> bool {
        match tokio::time::timeout(Duration::from_secs(5), self.ping()).await {
            Ok(r) => match r {
                Ok(true) => true,
                Ok(false) => false,
                Err(e) => {
                    warn!("failed alive check: {e}");
                    false
                }
            },
            Err(_) => false,
        }
    }

    #[cfg(not(feature = "mock"))]
    pub(crate) fn ciphers(&self) -> Cipher {
        self.ciphers.clone()
    }

    #[cfg(feature = "mock")]
    pub fn ciphers(&self) -> Cipher {
        self.ciphers.clone()
    }

    pub async fn subscribe(&self, topic: Topic) -> Result<SubscriptionId> {
        Ok(self.relay.subscribe(topic).await?)
    }

    pub fn actors(&self) -> Actors {
        self.actors.clone()
    }

    pub fn pair_key(&self) -> Option<String> {
        self.ciphers.public_key_hex()
    }

    pub fn topic(&self) -> Option<Topic> {
        self.ciphers.pairing().map(|p| p.topic.clone())
    }

    pub fn pairing(&self) -> Option<Pairing> {
        self.ciphers.pairing()
    }

    pub async fn ping(&self) -> Result<bool> {
        let t = self.topic().ok_or(Error::NoPairingTopic)?;
        self.transport
            .publish_request::<bool>(t, RequestParams::PairPing(PairPingRequest::default()))
            .await
    }

    pub(crate) fn find_session(&self, namespaces: &Namespaces) -> Option<SessionSettled> {
        if self.topic().is_none() {
            return None;
        }
        let settlements = self.ciphers.settlements().unwrap_or_default();
        if settlements.is_empty() {
            return None;
        }
        let required_chains = namespaces.chains();
        info!("required chains {}", required_chains);
        for s in settlements {
            let settled_chains = (&s).namespaces.chains();
            info!("settled chains {}", settled_chains);
            if required_chains.is_subset(&settled_chains) {
                return Some(s);
            }
        }
        None
    }

    pub async fn delete(&self) -> Result<bool> {
        let t = self.topic().ok_or(Error::NoPairingTopic)?;
        let result = self
            .transport
            .publish_request::<bool>(
                t.clone(),
                RequestParams::PairDelete(PairDeleteRequest::default()),
            )
            .await;
        self.cleanup(t).await;
        result
    }

    // Epoch
    pub async fn extend(&self, expiry: u64) -> Result<bool> {
        let t = self.topic().ok_or(Error::NoPairingTopic)?;
        self.transport
            .publish_request::<bool>(
                t.clone(),
                RequestParams::PairExtend(PairExtendRequest { expiry }),
            )
            .await
    }

    pub async fn set_pairing(&self, pairing: Pairing) -> Result<()> {
        if let Some(p) = self.pairing() {
            if p.topic == pairing.topic {
                return Ok(());
            }
        }
        self.ciphers.set_pairing(Some(pairing.clone()))?;
        self.subscribe(pairing.topic).await?;
        Ok(())
    }

    pub async fn publish_request<R: DeserializeOwned>(&self, params: RequestParams) -> Result<R> {
        let topic = self.topic().ok_or(Error::NoPairingTopic)?;
        self.transport.publish_request(topic, params).await
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.disconnect_socket().await
    }

    #[tracing::instrument(level = "info")]
    pub async fn open_socket(&self) -> Result<()> {
        self.relay.connect(&self.opts).await?;
        Ok(())
    }

    pub async fn disconnect_socket(&self) -> Result<()> {
        info!("closing websocket");
        if let Err(err) = self.relay.disconnect().await {
            warn!("failed to close socket {err}");
        }
        Ok(())
    }

    pub(crate) fn topic_transport(&self) -> TopicTransport {
        self.transport.clone()
    }
}

#[cfg(test)]
mod test {
    use walletconnect_namespaces::*;

    #[test]
    fn test_namespace_compare() {
        let settled_chains = Chains::from([
            ChainId::Solana(ChainType::Test),
            ChainId::EIP155(AlloyChain::sepolia()),
            ChainId::EIP155(AlloyChain::holesky()),
        ]);
        let required_chains = Chains::from([
            ChainId::Solana(ChainType::Test),
            ChainId::EIP155(alloy_chains::Chain::sepolia()),
        ]);
        assert!(required_chains.is_subset(&settled_chains));

        let settled_chains = Chains::from([ChainId::Solana(ChainType::Main)]);
        assert!(!required_chains.is_subset(&settled_chains));

        let settled_chains = Chains::from([
            ChainId::Solana(ChainType::Test),
            ChainId::EIP155(AlloyChain::sepolia()),
        ]);
        let required_chains = Chains::from([
            ChainId::Solana(ChainType::Test),
            ChainId::EIP155(AlloyChain::sepolia()),
            ChainId::EIP155(AlloyChain::holesky()),
        ]);
        assert!(!required_chains.is_subset(&settled_chains));
    }
}
