use crate::actors::{CipherActor, DeleteSession, SessionSettled};
use crate::rpc::{RequestParams, SessionDeleteRequest};
use crate::transport::SessionTransport;
use crate::{Error, PairingManager, PairingTopic, SessionEvent, SessionHandlers, Topic};
use crate::{Result, SessionDeleteHandler};
use dashmap::DashMap;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tokio::sync::oneshot::Sender;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, warn};
use walletconnect_namespaces::Namespaces;
use xtra::prelude::*;

mod session_delete;
mod session_ping;

pub(crate) struct HandlerContainer {
    pub tx: Sender<Result<ClientSession>>,
    pub handlers: Arc<Box<dyn SessionHandlers>>,
}

#[derive(Clone, Default)]
pub(crate) struct PendingSession {
    pending: Arc<DashMap<PairingTopic, HandlerContainer>>,
}

impl PendingSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add<T: SessionHandlers>(
        &self,
        topic: PairingTopic,
        handlers: T,
    ) -> oneshot::Receiver<Result<ClientSession>> {
        let (tx, rx) = oneshot::channel::<Result<ClientSession>>();
        let h = HandlerContainer {
            tx,
            handlers: Arc::new(Box::new(handlers)),
        };
        self.pending.insert(topic, h);
        rx
    }

    pub fn error(&self, topic: &PairingTopic, err: Error) {
        match self.remove(topic) {
            Ok(handlers) => {
                if handlers.tx.send(Err(err)).is_err() {
                    warn!("settlement channel has closed! {topic}");
                }
            }
            Err(_) => tracing::warn!("failed to find pairing topic {topic} in pending handlers"),
        };
    }
    fn remove(&self, topic: &PairingTopic) -> Result<HandlerContainer> {
        let (_, handler) = self
            .pending
            .remove(topic)
            .ok_or(crate::Error::InvalidPendingHandler(topic.clone()))?;
        Ok(handler)
    }

    pub async fn settled(
        &self,
        mgr: &PairingManager,
        settled: SessionSettled,
        send_to_peer: bool,
    ) -> Result<ClientSession> {
        let pairing_topic = mgr.topic().ok_or(Error::NoPairingTopic)?;
        let handlers = self.remove(&pairing_topic)?;
        let actors = mgr.actors();
        let session_transport = SessionTransport {
            topic: settled.0.clone(),
            transport: mgr.topic_transport(),
        };
        let (tx, rx) = mpsc::unbounded_channel::<SessionEvent>();
        let client_session = ClientSession::new(
            actors.cipher_actor(),
            session_transport,
            settled.1.namespaces.clone(),
            tx,
        );
        let req = settled.1.clone();
        actors
            .register_settlement(client_session.clone(), settled)
            .await?;
        if send_to_peer {
            client_session
                .publish_request::<bool>(RequestParams::SessionSettle(req))
                .await?;
        }
        handlers
            .tx
            .send(Ok(client_session.clone()))
            .map_err(|_| Error::SettlementRecvError)?;
        Ok(client_session)
    }
}

/// https://specs.walletconnect.com/2.0/specs/clients/sign/session-proposal
///
/// New session as the result of successful session proposal.
#[derive(Clone, Actor)]
pub struct ClientSession {
    pub namespaces: Arc<Namespaces>,
    transport: SessionTransport,
    cipher_actor: Address<CipherActor>,
    tx: mpsc::UnboundedSender<SessionEvent>,
}

impl ClientSession {
    pub(crate) fn new(
        cipher_actor: Address<CipherActor>,
        transport: SessionTransport,
        namespaces: Namespaces,
        tx: mpsc::UnboundedSender<SessionEvent>,
    ) -> Self {
        Self {
            transport,
            namespaces: Arc::new(namespaces),
            cipher_actor,
            tx,
        }
    }
}

impl ClientSession {
    pub fn namespaces(&self) -> &Namespaces {
        &self.namespaces
    }

    pub fn topic(&self) -> Topic {
        self.transport.topic.clone()
    }

    pub async fn publish_request<R: DeserializeOwned>(&self, params: RequestParams) -> Result<R> {
        self.transport.publish_request(params).await
    }

    pub async fn ping(&self) -> Result<bool> {
        self.publish_request(RequestParams::SessionPing(())).await
    }

    pub async fn delete(&self) -> Result<bool> {
        let accepted: bool = match self
            .publish_request(RequestParams::SessionDelete(SessionDeleteRequest::default()))
            .await
        {
            Ok(false) => {
                warn!("other side did not accept our delete request");
                false
            }
            Ok(true) => true,
            Err(e) => {
                error!("failed send session delete: {e}");
                false
            }
        };
        self.cleanup_session().await?;
        Ok(accepted)
    }

    async fn cleanup_session(&self) -> Result<()> {
        self.transport.unsubscribe().await?;
        self.cipher_actor
            .send(DeleteSession(self.topic()))
            .await??;
        Ok(())
    }
}
