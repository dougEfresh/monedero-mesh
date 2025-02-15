use {
    crate::{ConnectionHandler, ConnectionOptions, Result, SubscriptionId, Topic},
    reown_relay_client::{
        websocket::{Client as WcClient, ConnectionHandler as WcHandler, PublishedMessage},
        ConnectionOptions as WcOptions,
    },
    std::{
        fmt::{Debug, Display, Formatter},
        sync::Arc,
        time::Duration,
    },
};

impl From<&ConnectionOptions> for WcOptions {
    fn from(opts: &ConnectionOptions) -> Self {
        Self {
            address: String::from(&opts.address),
            project_id: opts.project_id.clone(),
            auth: opts.auth.clone(),
            origin: None,
            user_agent: None,
        }
    }
}

#[derive(Clone)]
pub struct Client {
    wc: WcClient,
}

impl Debug for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[native]")
    }
}

impl Display for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[native]")
    }
}

struct WrapperHandler<T: ConnectionHandler> {
    handler: T,
}

impl<T: ConnectionHandler> WrapperHandler<T> {
    const fn new(handler: T) -> Self {
        Self { handler }
    }
}

impl<T: ConnectionHandler> WcHandler for WrapperHandler<T> {
    fn connected(&mut self) {
        self.handler.connected();
    }

    fn disconnected(&mut self, _frame: Option<reown_relay_client::websocket::CloseFrame<'static>>) {
        self.handler.disconnected(None);
    }

    fn message_received(&mut self, message: PublishedMessage) {
        self.handler.message_received(message.into());
    }

    fn inbound_error(&mut self, err: reown_relay_client::error::ClientError) {
        self.handler.inbound_error(err.into());
    }

    fn outbound_error(&mut self, err: reown_relay_client::error::ClientError) {
        self.handler.outbound_error(err.into());
    }
}

impl Client {
    pub fn new(handler: impl ConnectionHandler) -> Self {
        let wrapper = WrapperHandler::new(handler);
        let wc = WcClient::new(wrapper);
        Self { wc }
    }
}

impl Client {
    /// Publishes a message over the network on given topic.
    pub async fn publish(
        &self,
        topic: Topic,
        message: impl Into<Arc<str>> + Send,
        tag: u32,
        ttl: Duration,
        prompt: bool,
    ) -> Result<()> {
        self.wc
            .publish(topic, message, None, tag, ttl, prompt)
            .await?;
        Ok(())
    }

    /// Subscribes on topic to receive messages.
    /// The request is resolved optimistically as soon as the relay receives it.
    pub async fn subscribe(&self, topic: Topic) -> Result<SubscriptionId> {
        let id = self.wc.subscribe(topic).await?;
        Ok(id)
    }

    /// Subscribes on multiple topics to receive messages. The request is
    /// resolved optimistically as soon as the relay receives it.
    pub async fn batch_subscribe(
        &self,
        topics: impl Into<Vec<Topic>> + Send,
    ) -> Result<Vec<SubscriptionId>> {
        let topics = self.wc.batch_subscribe(topics).await?;
        Ok(topics)
    }

    /// Unsubscribes from a topic
    pub async fn unsubscribe(&self, topic: Topic) -> Result<()> {
        self.wc.unsubscribe(topic).await?;
        Ok(())
    }

    pub async fn connect(&self, opts: &ConnectionOptions) -> Result<()> {
        let wc: WcOptions = WcOptions {
            address: String::from(&opts.address),
            project_id: opts.project_id.clone(),
            auth: opts.auth.clone(),
            origin: None,
            user_agent: None,
        };
        self.wc.connect(&wc).await?;
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<()> {
        self.wc.disconnect().await.ok();
        Ok(())
    }
}
