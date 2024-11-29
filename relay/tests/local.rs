use {
    monedero_relay::{
        auth_token,
        Client,
        ClientError,
        CloseFrame,
        ConnectionHandler,
        ConnectionOptions,
        Message,
        ProjectId,
        Topic,
    },
    std::{
        collections::VecDeque,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
            Mutex,
            Once,
        },
        time::Duration,
    },
    tracing::info,
    tracing_subscriber::{fmt::format::FmtSpan, EnvFilter},
};

static INIT: Once = Once::new();

fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_target(true)
            .with_level(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    });
}

#[derive(Clone)]
pub(crate) struct DummyHandler {
    messages: Arc<Mutex<VecDeque<Message>>>,
    connected: Arc<Mutex<AtomicBool>>,
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
            lock.push_back(message)
        }
    }

    fn inbound_error(&mut self, _error: ClientError) {}

    fn outbound_error(&mut self, _error: ClientError) {}
}

#[tokio::test]
#[cfg(not(feature = "mock"))]
async fn test_local_client() -> anyhow::Result<()> {
    init_tracing();
    let handler = DummyHandler::new();
    let c = Client::new(handler.clone());
    let p = ProjectId::from("9d5b20b16777cc49100cf9df3649bd24");
    let opts = ConnectionOptions::new(p, auth_token("https://github.com/dougEfresh"));
    c.connect(&opts).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let topic = Topic::generate();
    let id = c.subscribe(topic.clone()).await?;
    info!("subscribed to topic {topic} with id {id}");
    tokio::time::sleep(Duration::from_secs(5)).await;
    c.unsubscribe(topic.clone()).await?;
    tokio::time::sleep(Duration::from_secs(2)).await;
    c.disconnect().await?;
    Ok(())
}
