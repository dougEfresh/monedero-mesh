use {
    crate::{
        rpc::{RequestParams, SessionSettleRequest},
        session::Category,
        transport::SessionTransport,
        ClientSession,
        Error,
        PairingManager,
        Result,
        SessionHandler,
    },
    dashmap::DashMap,
    monedero_domain::{PairingTopic, SessionSettled},
    std::{sync::Arc, time::Duration},
    tokio::{
        sync::{
            oneshot::{self, Sender},
            Mutex,
        },
        time::timeout,
    },
    tracing::warn,
};

pub struct HandlerContainer {
    pub tx: Sender<Result<ClientSession>>,
    pub handlers: Arc<Mutex<Box<dyn SessionHandler>>>,
}

#[derive(Clone, Default)]
pub struct PendingSession {
    pending: Arc<DashMap<PairingTopic, HandlerContainer>>,
}

impl PendingSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add<T: SessionHandler>(
        &self,
        topic: PairingTopic,
        handlers: T,
    ) -> oneshot::Receiver<Result<ClientSession>> {
        let (tx, rx) = oneshot::channel::<Result<ClientSession>>();
        let h = HandlerContainer {
            tx,
            handlers: Arc::new(Mutex::new(Box::new(handlers))),
        };
        self.pending.insert(topic, h);
        rx
    }

    pub fn error(&self, topic: &PairingTopic, err: Error) {
        if let Ok(handlers) = self.remove(topic) {
            if handlers.tx.send(Err(err)).is_err() {
                warn!("settlement channel has closed! {topic}");
            }
        } else {
            warn!("failed to find pairing topic {topic} in pending handlers");
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
        settled: SessionSettled,
        category: Category,
        send_to_peer: Option<SessionSettleRequest>,
    ) -> Result<ClientSession> {
        let pairing_topic = mgr.topic().ok_or(Error::NoPairingTopic)?;
        let handlers = self.remove(&pairing_topic)?;
        let actors = mgr.actors();
        let session_transport = SessionTransport {
            topic: settled.topic.clone(),
            transport: mgr.topic_transport(),
        };
        let client_session = ClientSession::new(
            actors.session(),
            session_transport,
            settled.clone(),
            handlers.handlers,
            category,
        )
        .await?;
        // sanity check on connection
        if let Err(e) = timeout(Duration::from_secs(5), client_session.ping()).await {
            warn!("failed to ping session: {e}. Session maybe broken, try new pairing");
        }
        if let Some(req) = send_to_peer {
            let result = client_session
                .publish_request::<bool>(RequestParams::SessionSettle(req))
                .await;
            let client_session_result: Result<ClientSession> = match result {
                Ok(true) => Ok(client_session.clone()),
                Ok(false) => Err(Error::ProposalRejected),
                Err(e) => Err(e),
            };
            if handlers.tx.send(client_session_result).is_err() {
                warn!("oneshot proposal channel has closed");
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
