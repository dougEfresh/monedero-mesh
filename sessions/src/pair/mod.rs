mod builder;
mod handlers;
mod pairing;
mod registration;
mod socket_handler;

pub use builder::ReownBuilder;
use {
    crate::{
        actors::Actors,
        relay::RelayHandler,
        rpc::{PairDeleteRequest, PairExtendRequest, PairPingRequest, RequestParams},
        spawn_task,
        transport::TopicTransport,
        wait,
        Error,
        Result,
        SocketEvent,
        SocketListener,
    },
    monedero_cipher::Cipher,
    monedero_domain::{namespaces::Namespaces, Pairing, SessionSettled, SubscriptionId, Topic},
    monedero_relay::{Client, ConnectionOptions},
    serde::de::DeserializeOwned,
    std::{
        fmt::{Debug, Formatter},
        sync::Arc,
    },
    tokio::sync::mpsc,
    tracing::{info, warn},
};

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
            .map_or_else(|| String::from("none"), |t| crate::shorten_topic(&t));
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
        spawn_task(socket_handler::handle_socket(socket_handler, socket_rx));
        mgr.open_socket().await?;
        mgr.restore_saved_pairing().await?;
        Ok(mgr)
    }

    pub async fn register_socket_listener<T: SocketListener>(&self, listener: T) {
        let mut l = self.socket_listeners.lock().await;
        l.push(Box::new(listener));
    }

    pub(crate) async fn resubscribe(&self) -> Result<()> {
        self.pairing().ok_or(Error::NoPairingTopic)?;
        let topics = self.ciphers.subscriptions();
        self.relay.batch_subscribe(topics).await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) async fn unsubscribe_all(&self) -> Result<()> {
        self.pairing().ok_or(Error::NoPairingTopic)?;
        let topics = self.ciphers.subscriptions();
        for topic in topics {
            let _ = self.relay.unsubscribe(topic).await;
        }
        Ok(())
    }

    /// Check if other side is "alive"i
    /// If the peer returns an RPC error then it is "alive"
    /// Error only for network communication errors or relay server is down
    pub(crate) async fn alive(&self) -> bool {
        (wait::wait_until(5000, self.ping()).await).map_or(false, |r| match r {
            Ok(true) => true,
            Ok(false) => false,
            Err(e) => {
                warn!("failed alive check: {e}");
                false
            }
        })
    }

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
        self.ciphers.pairing().map(|p| p.topic)
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
        self.topic()?;
        let settlements = self.ciphers.settlements().unwrap_or_default();
        if settlements.is_empty() {
            return None;
        }
        let required_chains = namespaces.chains();
        info!("required chains {}", required_chains);
        for s in settlements {
            let settled_chains = s.namespaces.chains();
            info!("settled chains {}", settled_chains);
            if required_chains.is_subset(&settled_chains) {
                return Some(s);
            }
        }
        None
    }

    pub async fn delete(&self) -> Result<bool> {
        let t = self.topic().ok_or(Error::NoPairingTopic)?;
        let result = wait::wait_until(
            1100,
            self.transport.publish_request::<bool>(
                t.clone(),
                RequestParams::PairDelete(PairDeleteRequest::default()),
            ),
        )
        .await;
        self.cleanup(t).await;
        Ok(result.is_ok())
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
    use monedero_domain::namespaces::*;

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
