mod builder;
mod handlers;

use crate::actors::Actors;
use crate::domain::{SubscriptionId, Topic};
use crate::relay::RelayHandler;
use crate::rpc::{ErrorParams, PairExtendRequest, RequestParams, SessionSettleRequest};
use crate::transport::{SessionTransport, TopicTransport};
use crate::{Cipher, ClientSession, Pairing, Result};
pub use builder::WalletConnectBuilder;
use serde::de::DeserializeOwned;
use std::collections::BTreeSet;
use std::ops::Deref;
use tracing::{info, warn};
use walletconnect_relay::{Client, ConnectionOptions};
use xtra::prelude::*;

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
        actors.register_socket_handler(socket_handler).await?;
        mgr.open_socket().await?;
        Ok(mgr)
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

    pub async fn ping(&self) -> Result<bool> {
        let t = self.topic().ok_or(crate::Error::NoPairingTopic)?;
        self.transport
            .publish_request(t, RequestParams::PairPing(Default::default()))
            .await
    }

    pub async fn delete(&self) -> Result<bool> {
        let t = self.topic().ok_or(crate::Error::NoPairingTopic)?;
        self.transport
            .publish_request::<bool>(t.clone(), RequestParams::PairDelete(Default::default()))
            .await?;
        self.cleanup(t).await?;
        Ok(true)
    }

    /*
        pub(crate) fn find_session(
            &self,
            required: &ProposeNamespaces,
        ) -> Result<Option<(SessionTransport, SettleNamespaces)>> {
            if self.topic().is_none() {
                return Ok(None);
            }

            let settlements = self.ciphers.settlements()?;

            if settlements.is_empty() {
                return Ok(None);
            }
            let required_namespaces: BTreeSet<String> = required.deref().keys().cloned().collect();
            for s in settlements.into_iter() {
                let settlement = s.1.namespaces.clone();
                let topic = s.0.clone();
                let settled_namespaces: BTreeSet<String> =
                    s.1.namespaces.deref().keys().cloned().collect();
                if required_namespaces != settled_namespaces {
                    continue;
                }
                for ns in &required_namespaces {
                    let settled_space = s.1.namespaces.get(ns).unwrap();
                    let required_space = required.deref().get(ns).unwrap();
                    let settled_chains: Vec<String> = settled_space
                        .accounts
                        .iter()
                        .map(|a| {
                            let parts: Vec<&str> = a.split(":").collect();
                            let mut chain: String = String::from("unknown");
                            if parts.len() == 3 {
                                chain = format!("{}:{}", parts[0], parts[1]);
                            }
                            chain
                        })
                        .collect();
                    let settled_chains: BTreeSet<String> = settled_chains.into_iter().collect();
                    if required_space.chains.eq(&settled_chains) {
                        let transport = SessionTransport {
                            topic,
                            transport: self.transport.clone(),
                        };
                        return Ok(Some((transport, settlement)));
                    }
                }
            }
            Ok(None)
        }
    */

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
        let cipher = self.actors.cipher_actor();
        cipher.send(pairing.clone()).await??;
        self.actors
            .register_mgr(pairing.topic.clone(), self.clone())
            .await?;
        self.subscribe(pairing.topic.clone()).await?;
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
