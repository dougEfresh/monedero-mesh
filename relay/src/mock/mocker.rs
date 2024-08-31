use crate::mock::{ConnectionPair, ConnectionState};
use crate::{
    ClientError, ConnectionHandler, Message, MessageIdGenerator, Result, SubscriptionId, Topic,
};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::fmt::{Debug, Display, Formatter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, info, warn};

pub static DISCONNECT_TOPIC: Lazy<Topic> =
    Lazy::new(|| Topic::from("92b2701dbdbb72abea51591a06d41e7d76ebfe18e1a1ca5680a5ac6e3717c6d9"));

#[derive(Clone)]
pub struct Mocker {
    pub client_id: ConnectionPair,
    tx: mpsc::UnboundedSender<Message>,
    socket_tx: mpsc::UnboundedSender<ConnectionState>,
    topics: Arc<DashMap<Topic, SubscriptionId>>,
    pending: Arc<RwLock<VecDeque<Message>>>,
    connected: Arc<AtomicBool>,
    generator: MessageIdGenerator,
}

impl Mocker {
    pub fn new<T: ConnectionHandler>(
        handler: T,
        generator: MessageIdGenerator,
        client_id: ConnectionPair,
        tx: mpsc::UnboundedSender<Message>,
        rx: mpsc::UnboundedReceiver<Message>,
    ) -> Self {
        let (socket_tx, socket_rx) = mpsc::unbounded_channel::<ConnectionState>();
        let mocker = Self {
            client_id,
            tx,
            socket_tx,
            topics: Arc::new(DashMap::default()),
            connected: Arc::new(AtomicBool::new(false)),
            pending: Arc::new(RwLock::new(VecDeque::new())),
            generator,
        };
        let event_handler = mocker.clone();
        tokio::spawn(async move {
            event_loop(rx, socket_rx, event_handler, handler).await;
        });
        mocker
    }

    fn my_topic(&self, topic: &Topic) -> bool {
        self.topics.contains_key(topic)
    }

    fn fmt_common(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.client_id,)
    }
}

impl Debug for Mocker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt_common(f)
    }
}

impl Display for Mocker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt_common(f)
    }
}

impl Mocker {
    #[tracing::instrument(level = "info", skip(message))]
    #[allow(clippy::missing_errors_doc)]
    pub async fn publish(
        &self,
        topic: Topic,
        message: impl Into<Arc<str>> + Send,
        tag: u32,
        ttl: Duration,
        prompt: bool,
    ) -> Result<()> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err(ClientError::Disconnected);
        }
        if !self.my_topic(&topic) {
            return Err(ClientError::NotSubscribed(topic));
        }
        let msg = Message {
            id: self.generator.next(),
            subscription_id: SubscriptionId::generate(),
            topic: topic.clone(),
            message: message.into(),
            tag,
            published_at: chrono::DateTime::default(),
            received_at: chrono::DateTime::default(),
        };
        if let Err(e) = self.tx.send(msg) {
            warn!("{} mock channel is done {e}", self);
            return Err(ClientError::TxSendError);
        }
        Ok(())
    }

    #[tracing::instrument(level = "info")]
    #[allow(clippy::missing_errors_doc)]
    pub async fn subscribe(&self, topic: Topic) -> Result<SubscriptionId> {
        if topic.value() == DISCONNECT_TOPIC.value() {
            info!("{} forcing disconnect", self);
            let c = self.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                c.disconnect().await
            });
        }
        let id = SubscriptionId::generate();
        self.topics.insert(topic, id.clone());
        let mocker = self.clone();
        tokio::spawn(async move { pending_messages(mocker).await });
        Ok(id)
    }

    #[tracing::instrument(level = "info")]
    #[allow(clippy::missing_errors_doc)]
    pub async fn unsubscribe(&self, topic: Topic) -> Result<()> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err(ClientError::Disconnected);
        }
        self.topics.remove(&topic);
        Ok(())
    }

    #[tracing::instrument(level = "info")]
    pub fn connect_state(&self, state: ConnectionState) -> Result<()> {
        if let Err(e) = self.socket_tx.send(state) {
            warn!("{} failed to broadcast connection state {e}", self);
            return Err(ClientError::TxSendError);
        }
        Ok(())
    }

    #[allow(clippy::unused_async)]
    #[allow(clippy::missing_errors_doc)]
    pub async fn connect(&self) -> Result<()> {
        self.connect_state(ConnectionState::Open)
    }

    #[allow(clippy::unused_async)]
    #[allow(clippy::missing_errors_doc)]
    pub async fn disconnect(&self) -> Result<()> {
        self.connect_state(ConnectionState::Closed)
    }
}

async fn event_loop<T: ConnectionHandler>(
    mut rx: mpsc::UnboundedReceiver<Message>,
    mut socket_rx: mpsc::UnboundedReceiver<ConnectionState>,
    mocker: Mocker,
    mut handler: T,
) {
    info!("{} created mocker event loop", mocker);
    loop {
        select! {
          Some(state) = socket_rx.recv() => {
            match state {
              ConnectionState::Open => {
                        mocker.connected.store(true, Ordering::Relaxed);
                        handler.connected();
                    }
              ConnectionState::Closed => {
                        mocker.connected.store(false, Ordering::Relaxed);
                        info!("{} - sending disconnect", mocker);
                        handler.disconnected(None);
              }
            }
          },
          maybe_message = rx.recv() => {
            match maybe_message {
              None => return,
              Some(msg) => handler.message_received(msg)
            }
          }
        }
    }
}

async fn pending_messages(mocker: Mocker) {
    let mut w = mocker.pending.write().await;
    if (*w).is_empty() {
        return;
    }
    let pending: Vec<Message> = w.drain(..).collect();
    drop(w);
    info!(
        "{} sending {} messages from pending queue ",
        pending.len(),
        mocker
    );
    for m in pending {
        tokio::time::sleep(Duration::from_millis(800)).await;
        debug!("{} sending message", mocker);
        if mocker.tx.send(m).is_err() {
            warn!("{} mock channel closed", mocker);
            return;
        }
    }
}
