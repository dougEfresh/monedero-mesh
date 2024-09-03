mod builder;
mod handlers;
mod pairing;

use crate::actors::{Actors, RegisterComponent, RegisterPairing, SessionSettled};
use crate::domain::{SubscriptionId, Topic};
use crate::relay::RelayHandler;
use crate::rpc::{ErrorParams, PairExtendRequest, PairPingRequest, RequestParams, SessionSettleRequest};
use crate::transport::{SessionTransport, TopicTransport};
use crate::{Cipher, ClientSession, Pairing, Result};
pub use builder::WalletConnectBuilder;
use serde::de::DeserializeOwned;
use std::collections::BTreeSet;
use std::future::Future;
use std::ops::Deref;
use std::time::Duration;
use tracing::{info, warn};
use walletconnect_relay::{Client, ConnectionOptions};
use xtra::prelude::*;
use walletconnect_namespaces::Namespaces;

pub trait PairHandler: Send + 'static {
    fn ping(&mut self, topic: Topic);
    fn delete(&mut self, reason: ErrorParams, topic: Topic);
}

#[derive(Clone, xtra::Actor)]
pub struct PairingManager {
    relay: Client,
    opts: ConnectionOptions,
    ciphers: Cipher,
    transport: TopicTransport,
    actors: Actors,
}

impl PairingManager {
    async fn init(opts: ConnectionOptions, actors: Actors) -> Result<Self> {
        let ciphers = actors.cipher().clone();
        let handler = RelayHandler::new(ciphers.clone(), actors.clone());
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
        };

        let socket_handler = xtra::spawn_tokio(mgr.clone(), Mailbox::unbounded());
        info!("Opening connection to wc relay");
        mgr.open_socket().await?;
        actors.register_socket_handler(socket_handler).await?;
        mgr.restore_saved_pairing().await;
        Ok(mgr)
    }

    async fn restore_saved_pairing(&self) {
        if let Some(pairing) = self.pairing() {
            info!("found existing topic {pairing}");
            let r = RegisterPairing {
                pairing,
                mgr: self.clone(),
                component: RegisterComponent::None,
            };
            if let Err(e) = self.actors.register_pairing(r).await {
                warn!("failed to register pairing: {e}");
                return
            }
            info!("Checking if peer is alive");
            if !self.alive().await {
                info!("clearing pairing topics and sessions");
                self.actors.reset().await;
            }
        }
    }

    /// Check if other side is "alive"i
    /// If the peer returns an RPC error then it is "alive"
    /// Error only for network communication errors or relay server is down
    pub(crate) async fn alive(&self) -> bool {
        match tokio::time::timeout(Duration::from_secs(5), self.ping()).await {
            Ok(r) => match r {
                Ok(_) => true,
                Err(crate::Error::RpcError(_)) => true,
                Err(e) => {
                    warn!("failed alive check: {e}");
                    false
                }
            }
            Err(_) => false
        }
    }

    pub(crate) fn ciphers(&self) -> Cipher {
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
        let t = self.topic().ok_or(crate::Error::NoPairingTopic)?;
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
            let settled_chains = s.deref().namespaces.chains();
            info!("settled chains {}", settled_chains);
            if required_chains.is_subset(&settled_chains) {
                return Some(s)
            }
        }
        None
    }

    pub async fn delete(&self) -> Result<bool> {
        let t = self.topic().ok_or(crate::Error::NoPairingTopic)?;
        self.transport
            .publish_request::<bool>(t.clone(), RequestParams::PairDelete(Default::default()))
            .await?;
        self.cleanup(t).await?;
        Ok(true)
    }

    // Epoch
    pub async fn extend(&self, expiry: u64) -> Result<bool> {
        let t = self.topic().ok_or(crate::Error::NoPairingTopic)?;
        self.transport
            .publish_request::<bool>(
                t.clone(),
                RequestParams::PairExtend(PairExtendRequest { expiry }),
            )
            .await
    }

    pub async fn set_pairing(&self, pairing: Pairing) -> Result<()> {
        let register = RegisterPairing {
            pairing,
            mgr: self.clone(),
            component: RegisterComponent::None,
        };
        self.actors.register_pairing(register).await?;
        Ok(())
    }

    pub async fn publish_request<R: DeserializeOwned>(&self, params: RequestParams) -> Result<R> {
        let topic = self.topic().ok_or(crate::Error::NoPairingTopic)?;
        self.transport.publish_request(topic, params).await
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.disconnect_socket().await
    }

    pub async fn open_socket(&self) -> Result<()> {
        info!("opening websocket");
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
        let settled_chains = Chains::from([ChainId::Solana(ChainType::Test), ChainId::EIP155(AlloyChain::sepolia()), ChainId::EIP155(AlloyChain::holesky())]);
        let required_chains = Chains::from([ChainId::Solana(ChainType::Test), ChainId::EIP155(alloy_chains::Chain::sepolia())]);
        assert!(required_chains.is_subset(&settled_chains));

        let settled_chains = Chains::from([ChainId::Solana(ChainType::Main)]);
        assert!(!required_chains.is_subset(&settled_chains));

        let settled_chains = Chains::from([ChainId::Solana(ChainType::Test), ChainId::EIP155(AlloyChain::sepolia())]);
        let required_chains = Chains::from([ChainId::Solana(ChainType::Test), ChainId::EIP155(AlloyChain::sepolia()), ChainId::EIP155(AlloyChain::holesky())]);
        assert!(!required_chains.is_subset(&settled_chains));

    }
}