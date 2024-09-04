use crate::{Error, PairingManager, Result, SessionEvent, SessionTopic};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::oneshot::Sender;
use tokio::sync::{mpsc, oneshot};
use tracing::warn;

use crate::actors::SessionSettled;
use crate::rpc::{RequestParams, SessionSettleRequest};
use crate::transport::SessionTransport;
use crate::{ClientSession, PairingTopic, SessionHandlers};

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
            Err(_) => warn!("failed to find pairing topic {topic} in pending handlers"),
        };
    }
    fn remove(&self, topic: &PairingTopic) -> Result<HandlerContainer> {
        let (_, handler) = self
            .pending
            .remove(topic)
            .ok_or(Error::InvalidPendingHandler(topic.clone()))?;
        Ok(handler)
    }

    pub async fn settled(
        &self,
        mgr: &PairingManager,
        topic: SessionTopic,
        settled: SessionSettleRequest,
        send_to_peer: bool,
    ) -> Result<ClientSession> {
        let pairing_topic = mgr.topic().ok_or(Error::NoPairingTopic)?;
        let handlers = self.remove(&pairing_topic)?;
        let actors = mgr.actors();
        let session_transport = SessionTransport {
            topic,
            transport: mgr.topic_transport(),
        };
        let (tx, rx) = mpsc::unbounded_channel::<SessionEvent>();
        let client_session = ClientSession::new(
            mgr.ciphers(),
            session_transport,
            settled.namespaces.clone(),
            tx,
        );
        mgr.ciphers()
            .set_settlement(client_session.topic(), settled.clone())?;
        actors.session().send(client_session.clone()).await?;
        if send_to_peer {
            if let Err(e) = client_session
                .publish_request::<bool>(RequestParams::SessionSettle(settled))
                .await
            {
                let _ = handlers.tx.send(Err(Error::SettlementRejected));
                return Err(e);
            }
        } else {
            handlers
                .tx
                .send(Ok(client_session.clone()))
                .map_err(|_| Error::SettlementRecvError)?;
        }
        Ok(client_session)
    }
}
