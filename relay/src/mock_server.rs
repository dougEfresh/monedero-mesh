use {
    crate::Topic,
    dashmap::DashMap,
    futures_util::{SinkExt, StreamExt},
    std::{
        net::SocketAddr,
        sync::{Arc, Mutex},
    },
    tokio::{net::TcpListener, sync::oneshot, task::JoinHandle},
    tokio_tungstenite::{accept_async, tungstenite::Message},
    tracing::{error, info},
    walletconnect_sdk::rpc::rpc::{Payload, Response, SuccessfulResponse},
};

type Subscriptions = Arc<DashMap<u16, Vec<Topic>>>;

struct MockRelay {
    shutdown_tx: Option<oneshot::Sender<()>>,
    handle: Option<JoinHandle<()>>,
    pub subs: Subscriptions,
}

impl MockRelay {
    /// Starts the mock relay server and returns an instance of `MockRelay`.
    pub async fn start() -> crate::Result<Self> {
        let addr = "127.0.0.1:4000";
        let listener = TcpListener::bind(&addr).await?;

        // Channel to signal shutdown
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let subs: Subscriptions = Arc::new(DashMap::new());

        // Spawn the server task
        let handle = tokio::spawn(Self::run(listener, shutdown_rx, subs.clone()));

        Ok(Self {
            shutdown_tx: Some(shutdown_tx),
            handle: Some(handle),
            subs,
        })
    }

    /// The main server loop that accepts incoming connections.
    async fn run(
        listener: TcpListener,
        mut shutdown_rx: oneshot::Receiver<()>,
        subs: Subscriptions,
    ) {
        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, addr)) => {
                            let s = subs.clone();
                            tokio::spawn(async move {
                                Self::handle_connection(stream, addr, s).await;
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {e}");
                        }
                    }
                }
                _ = &mut shutdown_rx => {
                    info!("Shutting down the server.");
                    break;
                }
            }
        }
    }

    /// Handles individual WebSocket connections.
    async fn handle_connection(
        stream: tokio::net::TcpStream,
        addr: SocketAddr,
        subs: Subscriptions,
    ) {
        subs.insert(addr.port(), Vec::new());
        match accept_async(stream).await {
            Ok(ws_stream) => {
                let (mut ws_sender, mut ws_receiver) = ws_stream.split();
                while let Some(msg) = ws_receiver.next().await {
                    match msg {
                        Ok(msg) => {
                            let payload =
                                serde_json::from_str::<Payload>(msg.to_text().expect("no"));
                            match payload {
                                Ok(payload) => match payload {
                                    Payload::Request(request) => {
                                        info!("recv request {:?}", request);
                                        let payload: Payload = Payload::Response(
                                            Response::Success(SuccessfulResponse::new(
                                                request.id,
                                                serde_json::json!(true),
                                            )),
                                        );
                                        let payload =
                                            serde_json::to_string(&payload).expect("never");
                                        if ws_sender.send(Message::text(&payload)).await.is_err() {
                                            break;
                                        }
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

impl Drop for MockRelay {
    fn drop(&mut self) {
        // Send the shutdown signal
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        // Abort the server task
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{mock_connection_opts, Client, LogHandler, NoopHandler, ProjectId, Topic},
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

    #[tokio::test]
    async fn mock_server() -> anyhow::Result<()> {
        init_tracing();
        let server = MockRelay::start().await?;
        let client = Client::new(LogHandler::new(NoopHandler));
        let project_id = ProjectId::from("987f2292c12194ae69ddb6c52ceb1d62");
        let topic = Topic::generate();
        client.connect(&mock_connection_opts(project_id)).await?;
        let _ = client.subscribe(topic.clone()).await;
        let _ = client.publish(topic, message, 0, 10000, false);
        tokio::time::sleep(Duration::from_secs(1)).await;
        let _ = client.unsubscribe(topic.clone()).await;
        client.disconnect().await?;
        drop(server);
        Ok(())
    }
}
