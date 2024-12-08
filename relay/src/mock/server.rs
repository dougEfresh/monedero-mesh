use {
    super::{client::WsClient, PendingMessages, WsPublishedMessage},
    crate::MOCK_RELAY_ADDRESS,
    dashmap::DashSet,
    futures_util::{stream::SplitSink, SinkExt, StreamExt},
    serde::Serialize,
    std::{fmt::Debug, net::SocketAddr, sync::Arc, time::Duration},
    tokio::{
        net::{TcpListener, TcpStream},
        sync::Mutex,
    },
    tokio_tungstenite::{accept_async, tungstenite::Message, WebSocketStream},
    tracing::{debug, error, info, Level},
    walletconnect_sdk::{
        client::MessageIdGenerator,
        rpc::{
            domain::MessageId,
            rpc::{Payload, Response, SuccessfulResponse},
        },
    },
};

#[allow(dead_code)]
#[derive(Clone)]
pub struct MockRelay {
    pub(super) clients: Arc<DashSet<WsClient>>,
    pub(super) pending: PendingMessages,
    pub(super) tx: tokio::sync::broadcast::Sender<WsPublishedMessage>,
    pub(super) generator: MessageIdGenerator,
}

impl Debug for MockRelay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[mock-relay]({})", self.pending.len())
    }
}

impl MockRelay {
    /// Starts the mock relay server and returns an instance of `MockRelay`.
    pub async fn start() -> crate::Result<Self> {
        let listener = TcpListener::bind(MOCK_RELAY_ADDRESS).await?;
        let (tx, _rx) = tokio::sync::broadcast::channel::<WsPublishedMessage>(100);
        let me = Self {
            clients: Arc::new(DashSet::new()),
            pending: Arc::new(DashSet::new()),
            tx,
            generator: MessageIdGenerator::new(),
        };

        tokio::spawn(Self::run(me.clone(), listener));
        Ok(me)
    }

    /// The main server loop that accepts incoming connections.
    async fn run(relay: Self, listener: TcpListener) {
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

    pub async fn forward(
        payload: Payload,
        ws_sender: Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
        delay: Duration,
    ) {
        let payload = serde_json::to_string(&payload).expect("this should never happen");
        let mut l = ws_sender.lock().await;
        if let Err(e) = l.send(Message::text(&payload)).await {
            error!("client has closed connection {e}");
        }
        drop(l);
        tokio::time::sleep(delay).await;
    }

    pub async fn handle_ack<T: Serialize + Send>(
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
                let ws_client = WsClient::new(self, addr.port(), ws_sender);
                info!("created new ws client {ws_client}");
                self.clients.insert(ws_client);
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
                                    Payload::Request(_) => {
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
