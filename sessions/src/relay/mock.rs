use crate::domain::{SubscriptionId, Topic};
use crate::relay::{
    ClientError, CloseFrame, ConnectionHandler, ConnectionOptions, MessageIdGenerator, Result,
};
use crate::{relay, Atomic, Message};
use dashmap::DashMap;
use std::collections::{BTreeSet, VecDeque};
use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
type ClientId = Topic;

use once_cell::sync::Lazy;
use tracing::info;

pub(crate) static MOCK_FACTORY: Lazy<MockerFactory> = Lazy::new(|| MockerFactory::new());
// Special topic that indicates force disconnect
pub(crate) static DISCONNECT_TOPIC: Lazy<Topic> =
    Lazy::new(|| Topic::from("92b2701dbdbb72abea51591a06d41e7d76ebfe18e1a1ca5680a5ac6e3717c6d9"));
#[derive(Clone)]
pub(crate) struct Mocker {
    pub client_id: ClientId,
    tx: broadcast::Sender<MockPayload>,
    topics: Arc<DashMap<Topic, SubscriptionId>>,
    //pending: Arc<Mutex<VecDeque<Message>>>,
    connected: Arc<AtomicBool>,
    connect_event: MockPayload,
    disconnect_event: MockPayload,
    generator: MessageIdGenerator,
}

#[derive(Clone)]
pub(crate) struct MockPayload {
    id: ClientId,
    event: MockEvents,
}

fn display_client_id(id: &ClientId) -> String {
    let mut id = format!("{}", id);
    if id.len() > 10 {
        id = String::from(&id[0..9]);
    }
    id
}

impl Debug for MockPayload {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "clientId:{} event: {}",
            display_client_id(&self.id),
            self.event
        )
    }
}

impl Display for MockEvents {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let event = match self {
            MockEvents::Connect => String::from("connected"),
            MockEvents::Disconnect => String::from("disconnect"),
            MockEvents::Payload(p) => format!("messageId: {}", p.id),
            MockEvents::Shutdown => String::from("shutdown"),
        };
        write!(f, "{event}")
    }
}

#[derive(Clone)]
pub(crate) enum MockEvents {
    Connect,
    Disconnect,
    Payload(Message),
    //Subscribe(ClientId, Topic),
    Shutdown,
}

async fn event_loop<T: ConnectionHandler>(
    mut rx: broadcast::Receiver<MockPayload>,
    mocker: Mocker,
    mut handler: T,
) {
    tracing::info!("[{}] created mocker event loop", mocker);
    loop {
        match rx.recv().await {
            Err(_) => tracing::error!("got recv error for mock broadcast {mocker}"),
            Ok(payload) => match payload.event {
                MockEvents::Connect => {
                    if payload.id == mocker.client_id {
                        handler.connected();
                    }
                }
                MockEvents::Disconnect => {
                    if payload.id == mocker.client_id {
                        handler.disconnected(None)
                    }
                }
                MockEvents::Payload(message) => {
                    if payload.id == mocker.client_id {
                        tracing::debug!("[{}] got my own message", mocker);
                        continue;
                    }
                    if !mocker.connected.load(Ordering::Relaxed) {
                        tracing::debug!("[{}] not connected", mocker);
                        continue;
                    }
                    if !mocker.my_topic(&message.topic) {
                        tracing::debug!("[{}] subscribed to topic {}", mocker, message.topic);
                        continue;
                    }
                    handler.message_received(message);
                }
                MockEvents::Shutdown => break,
            },
        }
    }
}

#[derive(Clone, xtra::Actor)]
struct MockerActor {}

impl Mocker {
    pub fn new<T: ConnectionHandler>(
        handler: T,
        generator: MessageIdGenerator,
        tx: broadcast::Sender<MockPayload>,
    ) -> Self {
        let rx = tx.subscribe();
        let id = Topic::generate();
        let mocker = Self {
            client_id: id.clone(),
            tx,
            topics: Arc::new(Default::default()),
            connected: Arc::new(AtomicBool::new(false)),
            generator,
            connect_event: MockPayload {
                id: id.clone(),
                event: MockEvents::Connect,
            },
            disconnect_event: MockPayload {
                id,
                event: MockEvents::Disconnect,
            },
        };
        let event_handler = mocker.clone();
        tokio::spawn(async move {
            event_loop(rx, event_handler, handler).await;
        });
        mocker
    }

    fn my_topic(&self, topic: &Topic) -> bool {
        self.topics.contains_key(topic)
    }
}

pub struct MockerFactory {
    tx: broadcast::Sender<MockPayload>,
    generator: MessageIdGenerator,
}

impl MockerFactory {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            tx,
            generator: MessageIdGenerator::new(),
        }
    }

    pub fn create<T: ConnectionHandler>(&self, handler: T) -> Mocker {
        Mocker::new(handler, self.generator.clone(), self.tx.clone())
    }
}

impl Debug for Mocker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "clientId:{} topics: {}",
            display_client_id(&self.client_id),
            self.topics.len()
        )
    }
}

impl Display for Mocker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "clientId:{} topics: {}",
            display_client_id(&self.client_id),
            self.topics.len()
        )
    }
}

impl Mocker {
    #[tracing::instrument(level = "info", skip(message))]
    pub async fn publish(
        &self,
        topic: Topic,
        message: impl Into<Arc<str>>,
        tag: u32,
        _ttl: Duration,
        _prompt: bool,
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
            published_at: Default::default(),
            received_at: Default::default(),
        };
        let payload: MockPayload = MockPayload {
            event: MockEvents::Payload(msg),
            id: self.client_id.clone(),
        };
        if self.tx.send(payload).ok().is_none() {
            return Err(ClientError::TxSendError);
        }
        Ok(())
    }

    #[tracing::instrument(level = "info")]
    pub async fn subscribe(&self, topic: Topic) -> Result<SubscriptionId> {
        if topic.value() == DISCONNECT_TOPIC.value() {
            info!("forcing disconnect");
            let c = self.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                c.disconnect().await
            });
        }
        let id = SubscriptionId::generate();
        self.topics.insert(topic, id.clone());
        Ok(id)
    }

    #[tracing::instrument(level = "info")]
    pub async fn unsubscribe(&self, topic: Topic) -> Result<()> {
        self.topics.remove(&topic);
        Ok(())
    }

    #[tracing::instrument(level = "info")]
    pub fn connect_state(&self, mock_payload: MockPayload) -> Result<()> {
        match &mock_payload.event {
            MockEvents::Connect => self.connected.store(true, Ordering::Relaxed),
            MockEvents::Disconnect => self.connected.store(false, Ordering::Relaxed),
            _ => {}
        }
        if self.tx.send(mock_payload).ok().is_none() {
            return Err(ClientError::TxSendError);
        }
        Ok(())
    }

    pub async fn connect(&self) -> Result<()> {
        self.connect_state(self.connect_event.clone())
    }

    pub async fn disconnect(&self) -> Result<()> {
        self.connect_state(self.disconnect_event.clone())
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::domain::ProjectId;
    use crate::RELAY_ADDRESS;
    use assert_matches::assert_matches;
    use std::time::Duration;
    use tokio::time::sleep;
    use walletconnect_sdk::rpc::auth::ed25519_dalek::SigningKey;
    use walletconnect_sdk::rpc::auth::{AuthToken, SerializedAuthToken};

    pub(crate) struct TestClient {
        client: Mocker,
        handler: DummyHandler,
    }

    impl TestClient {
        pub(crate) fn new(factory: &MockerFactory) -> Self {
            let hdl = DummyHandler::new();
            let client = factory.create(hdl.clone());
            Self {
                client,
                handler: hdl,
            }
        }
    }

    #[derive(Clone)]
    pub(crate) struct DummyHandler {
        messages: Atomic<VecDeque<Message>>,
        connected: Atomic<AtomicBool>,
    }

    struct DummyHandlerRx {
        recv: std::sync::mpsc::Receiver<Message>,
        connected_rx: std::sync::mpsc::Receiver<bool>,
    }

    impl DummyHandlerRx {
        fn new(
            recv: std::sync::mpsc::Receiver<Message>,
            connected_rx: std::sync::mpsc::Receiver<bool>,
        ) -> Self {
            Self { recv, connected_rx }
        }
    }

    impl DummyHandler {
        pub(crate) fn new() -> Self {
            Self {
                messages: Arc::new(Mutex::new(VecDeque::new())),
                connected: Arc::new(Mutex::new(AtomicBool::new(false))),
            }
        }
    }

    impl DummyHandler {
        fn len(&self) -> anyhow::Result<usize> {
            let lock = self
                .messages
                .lock()
                .map_err(|_| anyhow::format_err!("failed to lock"))?;
            Ok(lock.len())
        }

        fn is_connected(&self) -> anyhow::Result<bool> {
            let lock = self
                .connected
                .lock()
                .map_err(|_| anyhow::format_err!("failed to lock"))?;
            Ok(lock.load(Ordering::Relaxed))
        }
    }

    impl ConnectionHandler for DummyHandler {
        #[tracing::instrument(level = "info", skip(self))]
        fn connected(&mut self) {
            let _ = self.connected.lock().is_ok_and(|guard| {
                guard.store(true, Ordering::Relaxed);
                true
            });
        }

        fn disconnected(&mut self, _frame: Option<CloseFrame<'static>>) {
            let _ = self.connected.lock().is_ok_and(|guard| {
                guard.store(false, Ordering::Relaxed);
                true
            });
        }

        fn message_received(&mut self, message: Message) {
            let l = self.messages.lock();
            if let Ok(mut lock) = l {
                lock.push_front(message)
            }
        }

        fn inbound_error(&mut self, _error: ClientError) {}

        fn outbound_error(&mut self, _error: ClientError) {}
    }

    pub(crate) fn auth() -> SerializedAuthToken {
        let key = SigningKey::generate(&mut rand::thread_rng());
        AuthToken::new("https://example.com")
            .aud(RELAY_ADDRESS)
            .ttl(Duration::from_secs(60 * 60))
            .as_jwt(&key)
            .unwrap()
    }

    pub(crate) fn connection_opts() -> ConnectionOptions {
        let auth = auth();
        let p = ProjectId::from("9d5b20b16777cc49100cf9df3649bd24");
        ConnectionOptions::new(p, auth)
    }

    fn setup() -> (TestClient, TestClient) {
        crate::tests::init_tracing();
        let factory = MockerFactory::new();
        let dapp = TestClient::new(&factory);
        let wallet = TestClient::new(&factory);
        (dapp, wallet)
    }

    #[tokio::test]
    async fn test_pub_sub() -> anyhow::Result<()> {
        let (mut dapp, mut wallet) = setup();
        let message: Message = Default::default();
        let ttl = Duration::from_secs(1);
        sleep(ttl).await;
        dapp.client.connect().await?;
        wallet.client.connect().await?;

        dapp.client.subscribe(message.topic.clone()).await?;
        wallet.client.subscribe(message.topic.clone()).await?;

        dapp.client
            .publish(
                message.topic.clone(),
                message.message.clone(),
                message.tag,
                ttl,
                false,
            )
            .await?;

        sleep(ttl).await;
        assert_eq!(1, wallet.handler.len()?);

        let topic = message.topic.clone();
        // new message but same topic
        let mut message = Message {
            topic: topic.clone(),
            ..Default::default()
        };
        wallet
            .client
            .publish(
                message.topic.clone(),
                message.message.clone(),
                message.tag,
                ttl,
                false,
            )
            .await?;

        sleep(ttl).await;
        assert_eq!(1, dapp.handler.len()?);
        wallet.client.unsubscribe(message.topic.clone()).await?;
        dapp.client
            .publish(
                message.topic.clone(),
                message.message.clone(),
                message.tag,
                ttl,
                false,
            )
            .await?;
        sleep(ttl).await;
        assert_eq!(1, wallet.handler.len()?);
        Ok(())
    }

    /// Note, I have to sleep to allow the callback handler to execute
    /// I would use channels but was running into wierd issues of closed channels
    #[tokio::test]
    async fn test_connection_state() -> anyhow::Result<()> {
        let (dapp, _) = setup();
        let message: Message = Default::default();
        let ttl = Duration::from_millis(1000);
        // give a moment for the event_loop to start
        tokio::time::sleep(ttl).await;
        let err = dapp
            .client
            .publish(
                message.topic.clone(),
                message.message.clone(),
                message.tag,
                ttl,
                false,
            )
            .await
            .unwrap_err();
        assert_matches!(err, ClientError::Disconnected);

        dapp.client.connect().await?;
        tokio::time::sleep(ttl).await;
        assert!(dapp.handler.is_connected()?);

        let err = dapp
            .client
            .publish(
                message.topic.clone(),
                message.message.clone(),
                message.tag,
                ttl,
                false,
            )
            .await
            .unwrap_err();
        assert_matches!(err, ClientError::NotSubscribed(topic) if topic == message.topic.clone());

        dapp.client.disconnect().await?;
        tokio::time::sleep(ttl).await;
        assert!(!dapp.handler.is_connected()?);

        let err = dapp
            .client
            .publish(
                message.topic.clone(),
                message.message.clone(),
                message.tag,
                ttl,
                false,
            )
            .await
            .unwrap_err();
        assert_matches!(err, ClientError::Disconnected);
        Ok(())
    }
}
