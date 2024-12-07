use {
    dashmap::DashSet,
    futures_util::stream::SplitSink,
    std::{
        fmt::{Debug, Display},
        sync::Arc,
    },
    tokio::{net::TcpStream, sync::Mutex},
    tokio_tungstenite::{tungstenite::Message, WebSocketStream},
    walletconnect_sdk::rpc::rpc::{Payload, Publish},
};

mod client;
mod server;
pub use server::MockRelay;

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

type PendingMessages = Arc<DashSet<Publish>>;
type WsSender = Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>;

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{
            default_connection_opts,
            mock_connection_opts,
            Client,
            ConnectionHandler,
            LogHandler,
            NoopHandler,
            ProjectId,
            Topic,
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
        // Verify the when I unsubscribe, and client_2 sends a message it is put in
        // "PendingMessages"
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
        assert_eq!(0, 0);
        let topics = vec![topic.clone(), Topic::generate(), Topic::generate()];
        client_1.batch_subscribe(topics.clone()).await?;
        yield_ms(100).await;
        let num_messages = { messages_1.read().expect("could not unlock").len() };
        assert_eq!(1, num_messages);
        client_1.disconnect().await?;
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
