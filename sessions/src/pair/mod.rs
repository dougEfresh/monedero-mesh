mod builder;
mod handlers;

use crate::actors::Actors;
use crate::domain::{SubscriptionId, Topic};
use crate::relay::relay_handler::RelayHandler;
use crate::relay::{Client, ConnectionOptions};
use crate::rpc::{ErrorParams, PairExtendRequest, RequestParams};
use crate::transport::TopicTransport;
use crate::{Cipher, Pairing, Result};
pub use builder::WalletConnectBuilder;
use serde::de::DeserializeOwned;
use tracing::{info, warn};
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
        let relay = Client::mock(handler);

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
        if let Err(_) = actors.register_socket_handler(socket_handler).await {
            warn!("failed to register socket handler!");
        }
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
        self.ciphers.set_pairing(None)?;
        self.relay.unsubscribe(t).await?;
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
