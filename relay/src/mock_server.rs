use {
    crate::Topic,
    dashmap::DashMap,
    futures_util::{stream::SplitSink, SinkExt, StreamExt},
    serde::{de::DeserializeOwned, Serialize},
    std::{collections::VecDeque, fmt::Display, net::SocketAddr, sync::Arc},
    tokio::{
        net::{TcpListener, TcpStream},
        sync::{
            broadcast::{Receiver, Sender},
            oneshot, Mutex, RwLock,
        },
        task::JoinHandle,
    },
    tokio_tungstenite::{accept_async, tungstenite::Message, WebSocketStream},
    tracing::{debug, error, info},
    walletconnect_sdk::rpc::{
        domain::{MessageId, SubscriptionId},
        rpc::{Params, Payload, Request, Response, SuccessfulResponse},
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
type TopicMap = Arc<DashMap<Topic, Sender<WsPublishedMessage>>>;

type PendingMessages = Arc<RwLock<VecDeque<crate::Message>>>;
type WsSender = Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>;
#[derive(Clone)]
struct WsClient {
    id: u16,
    topics: Arc<DashMap<Topic, bool>>,
    ws_sender: WsSender,
}

impl Display for WsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] ({})", self.id, self.topics.len())
    }
}

impl WsClient {
    pub(crate) fn new(id: u16, rx: Receiver<WsPublishedMessage>, ws_sender: WsSender) -> Self {
        let me = Self {
            id,
            ws_sender,
            topics: Arc::new(DashMap::new()),
        };
        let listener = me.clone();
        tokio::spawn(listener.run(rx));
        me
    }

    async fn run(self, mut rx: Receiver<WsPublishedMessage>) {
        while let Ok(published_message) = rx.recv().await {
            let id = published_message.payload.id();
            info!("new message {published_message}");
            if published_message.client_id == self.id {
                let topics: Vec<Topic> = match published_message.payload {
                    Payload::Request(req) => match req.params {
                        Params::Subscribe(s) => {
                            info!("subscribe request to {}", s.topic);
                            vec![s.topic]
                        }
                        Params::BatchSubscribe(b) => {
                            info!("batch sub");
                            b.topics
                        }
                        Params::Unsubscribe(s) => {
                            tokio::spawn(MockRelay::handle_ack(id, self.ws_sender.clone(), true));
                            self.topics.remove(&s.topic);
                            vec![]
                        }
                        Params::Publish(p) => {
                            tokio::spawn(MockRelay::handle_ack(id, self.ws_sender.clone(), true));
                            vec![]
                        }
                        _ => vec![],
                    },
                    Payload::Response(_) => vec![],
                };
                if !topics.is_empty() {
                    for t in topics {
                        self.topics.insert(t, true);
                    }
                    tokio::spawn(MockRelay::handle_ack(
                        id,
                        self.ws_sender.clone(),
                        SubscriptionId::generate(),
                    ));
                }
                continue;
            }
        }
    }
}

//pub subs: Subscriptions,
#[derive(Clone)]
struct MockRelay {
    clients: Arc<DashMap<u16, WsClient>>,
    pending: PendingMessages,
    tx: tokio::sync::broadcast::Sender<WsPublishedMessage>,
}

impl MockRelay {
    /// Starts the mock relay server and returns an instance of `MockRelay`.
    pub(crate) async fn start() -> crate::Result<Self> {
        let addr = "127.0.0.1:4000";
        let listener = TcpListener::bind(&addr).await?;
        let (tx, _rx) = tokio::sync::broadcast::channel::<WsPublishedMessage>(100);
        let me = Self {
            clients: Arc::new(DashMap::new()),
            pending: Arc::new(RwLock::new(VecDeque::new())),
            tx,
        };

        let topic_map: TopicMap = Arc::new(DashMap::new());
        let handle = tokio::spawn(Self::run(me.clone(), listener, topic_map));
        Ok(me)
    }

    /// The main server loop that accepts incoming connections.
    async fn run(relay: MockRelay, listener: TcpListener, topic_map: TopicMap) {
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

    async fn handle_pending_messages(
        ws_sender: Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
        pending: PendingMessages,
    ) {
        let mut w = pending.write().await;
        if (*w).is_empty() {
            return;
        }
        let pending: Vec<crate::Message> = w.drain(..).collect();
        drop(w);
        info!("{} sending from pending queue ", pending.len());
        for m in pending {
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            debug!("sending message");
            //let p: Payload = Payload::Request(Request {

            //});
            let payload: Payload = Payload::Response(Response::Success(SuccessfulResponse::new(
                m.id,
                serde_json::json!(true),
            )));
            let payload = serde_json::to_string(&payload).expect("never");
            let mut ws_sender_lk = ws_sender.lock().await;
            if ws_sender_lk.send(Message::text(&payload)).await.is_err() {
                error!("client has closed connection");
            }
            drop(ws_sender_lk);
        }
    }

    //fn handle_request(&self, client_id: u16, params: Params) {
    //    match params {
    //        Params::Subscribe(subscribe) => {
    //            let sub_id = SubscriptionId::generate();
    //            tokio::spawn(Self::handle_ack(message_id, ws_sender.clone(), sub_id));
    //
    //            self.handle_subscribe(ws_sender, client_id, subscribe.topic, topic_map, pending);
    //
    //            // TODO populate topic_map
    //        }
    //        Params::BatchSubscribe(_) => {
    //            info!("batch sub");
    //        }
    //        _ => {
    //            tokio::spawn(Self::handle_ack(message_id, ws_sender, true));
    //        }
    //    };
    //}
    //
    pub(crate) async fn handle_ack<T: Serialize + Send>(
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
    async fn handle_connection(&self, stream: tokio::net::TcpStream, addr: SocketAddr) {
        match accept_async(stream).await {
            Ok(ws_stream) => {
                let (ws_sender, mut ws_receiver) = ws_stream.split();
                let ws_sender = Arc::new(Mutex::new(ws_sender));
                let ws_client = WsClient::new(addr.port(), self.tx.subscribe(), ws_sender);
                info!("created new ws client {ws_client}");
                self.clients.insert(addr.port(), ws_client);
                while let Some(msg) = ws_receiver.next().await {
                    match msg {
                        Ok(msg) => {
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
//
//impl Drop for MockRelay {
//    fn drop(&mut self) {
//        // Send the shutdown signal
//        if let Some(shutdown_tx) = self.shutdown_tx.take() {
//            let _ = shutdown_tx.send(());
//        }
//
//        // Abort the server task
//        if let Some(handle) = self.handle.take() {
//            handle.abort();
//        }
//    }
//}
//
#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{mock_connection_opts, Client, LogHandler, NoopHandler, ProjectId, Topic},
        serde_json::json,
        std::time::Duration,
        tracing_subscriber::{fmt::format::FmtSpan, EnvFilter},
    };
    fn init_tracing() {
        tracing_subscriber::fmt()
            .with_target(true)
            .with_level(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn mock_server() -> anyhow::Result<()> {
        init_tracing();
        let server = MockRelay::start().await?;
        let client_1 = Client::new(LogHandler::new(NoopHandler));
        let client_2 = Client::new(LogHandler::new(NoopHandler));
        let project_id = ProjectId::from("987f2292c12194ae69ddb6c52ceb1d62");
        let topic = Topic::generate();
        client_1.connect(&mock_connection_opts(&project_id)).await?;
        client_2.connect(&mock_connection_opts(&project_id)).await?;
        client_1.subscribe(topic.clone()).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
        client_2
            .publish(
                topic.clone(),
                json!({"message": "blah"}).to_string(),
                0,
                Duration::from_secs(1),
                false,
            )
            .await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        client_1.unsubscribe(topic.clone()).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
        client_1.disconnect().await?;
        drop(server);
        Ok(())
    }
}
