use {
    crate::Topic,
    dashmap::DashMap,
    futures_util::{stream::SplitSink, SinkExt, StreamExt},
    serde::{de::DeserializeOwned, Serialize},
    std::{
        collections::VecDeque,
        fmt::{Debug, Display},
        net::SocketAddr,
        sync::Arc,
    },
    tokio::{
        net::{TcpListener, TcpStream},
        sync::{
            broadcast::{Receiver, Sender},
            oneshot, Mutex, RwLock,
        },
        task::JoinHandle,
    },
    tokio_tungstenite::{accept_async, tungstenite::Message, WebSocketStream},
    tracing::{debug, error, info, warn, Level},
    walletconnect_sdk::{
        client::MessageIdGenerator,
        rpc::{
            domain::{MessageId, SubscriptionId},
            rpc::{Params, Payload, Request, Response, SubscriptionData, SuccessfulResponse},
        },
    },
};

#[derive(Clone)]
struct WsPublishedMessage {
    client_id: u16, // port
    payload: Payload,
}
impl Display for WsPublishedMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {:?}", self.client_id, self.payload)
    }
}

impl Debug for WsPublishedMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[client_id:{}][message_id:{}]",
            self.client_id,
            self.payload.id()
        )
    }
}
type TopicMap = Arc<DashMap<Topic, Sender<WsPublishedMessage>>>;

//type PendingMessages = Arc<RwLock<VecDeque<crate::Message>>>;
type PendingMessages = Arc<DashMap<MessageId, WsPublishedMessage>>;
type WsSender = Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>;
#[derive(Clone)]
struct WsClient {
    id: u16,
    topics: Arc<DashMap<Topic, bool>>,
    ws_sender: WsSender,
    generator: MessageIdGenerator,
    pending: PendingMessages,
}

impl Display for WsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_common())
    }
}
impl Debug for WsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_common())
    }
}

impl WsClient {
    fn fmt_common(&self) -> String {
        format!("[wsclient-{}]({})", self.id, self.topics.len())
    }
    fn new(relay: &MockRelay, id: u16, ws_sender: WsSender) -> Self {
        let me = Self {
            id,
            ws_sender,
            topics: Arc::new(DashMap::new()),
            generator: relay.generator.clone(),
            pending: relay.pending.clone(),
        };
        let listener = me.clone();
        tokio::spawn(listener.handle_message(relay.tx.subscribe()));
        me
    }

    async fn handle_message(self, mut rx: Receiver<WsPublishedMessage>) {
        while let Ok(published_message) = rx.recv().await {
            let id = published_message.payload.id();
            if published_message.client_id == self.id {
                self.handle_own_message(id, published_message);
                continue;
            }
            self.handle_published_message(id, published_message);
        }
    }

    #[tracing::instrument(level = Level::DEBUG)]
    fn handle_published_message(&self, id: MessageId, published_message: WsPublishedMessage) {
        match &published_message.payload {
            Payload::Request(ref req) => {
                if let Params::Publish(ref p) = req.params {
                    if !self.topics.contains_key(&p.topic) {
                        warn!(
                            "got a message but I am not subscribed to this topic {}",
                            p.topic
                        );
                        return;
                    }
                    let forward_id = self.generator.next();
                    debug!(
                        "forwarding request from client_id:{} with new message_id: {forward_id}",
                        published_message.client_id,
                    );
                    let now = chrono::Utc::now().timestamp();
                    let subscription_id = SubscriptionId::from(p.topic.as_ref());
                    let sub_req = p.as_subscription_request(forward_id, subscription_id, now);
                    let forward_payload = Payload::Request(sub_req);
                    tokio::spawn(MockRelay::forward(forward_payload, self.ws_sender.clone()));
                } else {
                    debug!("not handling message");
                }
            }
            Payload::Response(res) => debug!("not handling response payload {:?}", res),
        };
    }

    #[tracing::instrument(level = Level::DEBUG)]
    fn handle_own_message(&self, id: MessageId, published_message: WsPublishedMessage) {
        debug!("handle my own message");
        match published_message.payload {
            Payload::Request(req) => match req.params {
                Params::Subscribe(s) => {
                    let sub_id = SubscriptionId::from(s.topic.as_ref());
                    debug!("subscribe request to subId:{} {}", sub_id, s.topic);
                    self.topics.insert(s.topic, true);
                    tokio::spawn(MockRelay::handle_ack(id, self.ws_sender.clone(), sub_id));
                }
                Params::BatchSubscribe(b) => {
                    debug!("batch sub");
                    let mut ids: Vec<SubscriptionId> = Vec::with_capacity(b.topics.len());
                    for t in b.topics {
                        ids.push(SubscriptionId::from(t.as_ref()));
                        self.topics.insert(t, true);
                    }
                    tokio::spawn(MockRelay::handle_ack(id, self.ws_sender.clone(), ids));
                }
                Params::Unsubscribe(s) => {
                    tokio::spawn(MockRelay::handle_ack(id, self.ws_sender.clone(), true));
                    self.topics.remove(&s.topic);
                }
                Params::Publish(p) => {
                    debug!("responding to my own published message");
                    tokio::spawn(MockRelay::handle_ack(id, self.ws_sender.clone(), true));
                }
                _ => {}
            },
            Payload::Response(_) => {}
        };
    }
}

//pub subs: Subscriptions,
#[derive(Clone)]
struct MockRelay {
    clients: Arc<DashMap<u16, WsClient>>,
    pending: PendingMessages,
    tx: tokio::sync::broadcast::Sender<WsPublishedMessage>,
    generator: MessageIdGenerator,
}

impl Debug for MockRelay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[mock-relay][{}]", self.clients.len())
    }
}

impl MockRelay {
    /// Starts the mock relay server and returns an instance of `MockRelay`.
    pub(crate) async fn start() -> crate::Result<Self> {
        let addr = "127.0.0.1:4000";
        let listener = TcpListener::bind(&addr).await?;
        let (tx, _rx) = tokio::sync::broadcast::channel::<WsPublishedMessage>(100);
        let me = Self {
            clients: Arc::new(DashMap::new()),
            //pending: Arc::new(RwLock::new(VecDeque::new())),
            pending: Arc::new(DashMap::new()),
            tx,
            generator: MessageIdGenerator::new(),
        };

        let topic_map: TopicMap = Arc::new(DashMap::new());
        let handle = tokio::spawn(Self::run(me.clone(), listener, topic_map));
        Ok(me)
    }

    /// The main server loop that accepts incoming connections.
    async fn run(relay: Self, listener: TcpListener, topic_map: TopicMap) {
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let me = relay.clone();
                    tokio::spawn(async move {
                        me.handle_connection(stream, addr).await;
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {e}");
                }
            };
        }
    }

    async fn forward(
        payload: Payload,
        ws_sender: Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
    ) {
        let mut ws_sender = ws_sender.lock().await;
        let payload = serde_json::to_string(&payload).expect("this should never happen");
        if ws_sender.send(Message::text(&payload)).await.is_err() {
            error!("client has closed connection");
        }
    }

    async fn handle_ack<T: Serialize + Send>(
        id: MessageId,
        ws_sender: Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
        result: T,
    ) {
        // resond back
        let payload: Payload = Payload::Response(Response::Success(SuccessfulResponse::new(
            id,
            serde_json::to_value(&result).expect("yo"),
        )));
        let payload = serde_json::to_string(&payload).expect("never");
        let mut ws_sender = ws_sender.lock().await;
        if ws_sender.send(Message::text(&payload)).await.is_err() {
            error!("client has closed connection");
        }
    }

    /// Handles individual WebSocket connections.
    #[tracing::instrument(level = Level::INFO, skip(stream, addr))]
    async fn handle_connection(&self, stream: tokio::net::TcpStream, addr: SocketAddr) {
        match accept_async(stream).await {
            Ok(ws_stream) => {
                let (ws_sender, mut ws_receiver) = ws_stream.split();
                let ws_sender = Arc::new(Mutex::new(ws_sender));
                let ws_client = WsClient::new(&self, addr.port(), ws_sender);
                info!("created new ws client {ws_client}");
                self.clients.insert(addr.port(), ws_client);
                while let Some(msg) = ws_receiver.next().await {
                    match msg {
                        Ok(msg) => {
                            if !msg.is_text() {
                                continue;
                            }
                            let payload =
                                serde_json::from_str::<Payload>(msg.to_text().expect("no"));
                            match payload {
                                Ok(payload) => match &payload {
                                    Payload::Request(request) => {
                                        let msg = WsPublishedMessage {
                                            client_id: addr.port(),
                                            payload,
                                        };
                                        debug!("broadcast payload from client id {}", addr.port());
                                        let _ = self.tx.send(msg);
                                    }
                                    Payload::Response(response) => {
                                        info!("recv response {:?}", response);
                                    }
                                },
                                Err(e) => {
                                    error!("invalid payload {e}");
                                }
                            };
                        }
                        Err(e) => {
                            error!("WebSocket error: {e}");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to upgrade to WebSocket: {e}");
            }
        }
        info!("Connection with {addr} closed.");
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{
            default_connection_opts, mock_connection_opts, Client, ConnectionHandler, LogHandler,
            NoopHandler, ProjectId, Topic,
        },
        serde_json::json,
        std::time::Duration,
        tracing_subscriber::{fmt::format::FmtSpan, EnvFilter},
    };

    async fn yield_ms(ms: u64) {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }

    type MockMessages = Arc<std::sync::RwLock<VecDeque<crate::Message>>>;
    struct EchoHandler {
        messages: MockMessages,
    }

    impl ConnectionHandler for EchoHandler {
        fn message_received(&mut self, message: crate::Message) {
            let mut l = self.messages.write().expect("cannot lock");
            l.push_back(message);
        }
    }

    fn init_tracing() {
        tracing_subscriber::fmt()
            .with_target(true)
            .with_level(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn mock_relay() -> anyhow::Result<()> {
        init_tracing();
        let server = MockRelay::start().await?;
        let messages_1: MockMessages = Arc::new(std::sync::RwLock::new(VecDeque::new()));
        let handler_1 = EchoHandler {
            messages: messages_1.clone(),
        };
        let client_1 = Client::new(handler_1);
        let client_2 = Client::new(LogHandler::new(NoopHandler));
        let project_id = ProjectId::from("987f2292c12194ae69ddb6c52ceb1d62");
        let topic = Topic::generate();
        client_1.connect(&mock_connection_opts(&project_id)).await?;
        client_2.connect(&mock_connection_opts(&project_id)).await?;
        client_1.subscribe(topic.clone()).await?;
        yield_ms(500).await;
        client_2
            .publish(
                topic.clone(),
                Arc::from("reown the world"),
                0,
                Duration::from_secs(60),
                false,
            )
            .await?;
        yield_ms(500).await;

        let num_messages = { messages_1.read().expect("could not unlock").len() };
        assert_eq!(1, num_messages);
        {
            messages_1.write().expect("cannot drain").clear();
        }
        // Verify the when I unsubscribe, and client_2 sends a message it is put in "PendingMessages"
        client_1.unsubscribe(topic.clone()).await?;
        client_2
            .publish(
                topic.clone(),
                Arc::from("reown the world"),
                0,
                Duration::from_secs(60),
                false,
            )
            .await?;
        yield_ms(100).await;
        let num_messages = { messages_1.read().expect("could not unlock").len() };
        assert_eq!(1, num_messages);
        {
            messages_1.write().expect("cannot drain").clear();
        }

        //let topics = vec![Topic::generate(), Topic::generate()];
        //client_1.batch_subscribe(topics.clone()).await?;
        //tokio::time::sleep(Duration::from_millis(500)).await;
        //for t in topics {
        //    client_1.unsubscribe(t).await?;
        //}
        //client_1.disconnect().await?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn real_relay() -> anyhow::Result<()> {
        let run = std::env::var("REAL_RELAY").ok();
        if run.is_none() {
            return Ok(());
        }
        init_tracing();
        let messages_1: MockMessages = Arc::new(std::sync::RwLock::new(VecDeque::new()));
        let handler_1 = EchoHandler {
            messages: messages_1.clone(),
        };
        let client_1 = Client::new(handler_1);
        let client_2 = Client::new(LogHandler::new(NoopHandler));
        let project_id = ProjectId::from("987f2292c12194ae69ddb6c52ceb1d62");
        let topic = Topic::generate();
        client_1
            .connect(&default_connection_opts(&project_id))
            .await?;
        client_2
            .connect(&default_connection_opts(&project_id))
            .await?;
        client_1.subscribe(topic.clone()).await?;
        yield_ms(500).await;
        client_2
            .publish(
                topic.clone(),
                Arc::from("reown everything"),
                0,
                Duration::from_secs(60),
                false,
            )
            .await?;
        {
            error!(
                "handler 1 has {} messages",
                messages_1.read().expect("could not unlock").len()
            );
        }
        Ok(())
    }
}
