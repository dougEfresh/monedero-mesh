mod error;
pub(crate) mod mock;
pub mod relay_handler;

use crate::domain::{Message, ProjectId, SubscriptionId, Topic};
use crate::RELAY_ADDRESS;
pub use error::ClientError;
use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;
use walletconnect_sdk::client::websocket::PublishedMessage;
pub use walletconnect_sdk::client::websocket::{
    Client as WcClient, ConnectionHandler as WcHandler,
};
use walletconnect_sdk::client::Authorization;
pub use walletconnect_sdk::client::ConnectionOptions as WcOptions;
pub use walletconnect_sdk::client::MessageIdGenerator;
use walletconnect_sdk::rpc::auth::SerializedAuthToken;

pub(crate) type Result<T> = std::result::Result<T, ClientError>;

#[derive(Debug, Clone)]
pub struct ConnectionOptions {
    /// The Relay websocket address. The default address is
    /// `wss://relay.walletconnect.com`.
    pub address: String,

    /// The project-specific secret key. Can be generated in the Cloud Dashboard
    /// at the following URL: <https://cloud.walletconnect.com/app>
    pub project_id: ProjectId,

    /// The authorization method and auth token to use.
    pub auth: Authorization,

    /// Optional origin of the request. Subject to allow-list validation.
    pub origin: Option<String>,

    /// Mock the client for internal loopback testing
    pub mock: bool, // Optional user agent parameters.
                    //pub user_agent: Option<UserAgent>,
}

impl ConnectionOptions {
    pub fn new(project_id: ProjectId, serialized: SerializedAuthToken) -> Self {
        Self {
            address: RELAY_ADDRESS.into(),
            project_id,
            auth: Authorization::Query(serialized),
            origin: None,
            mock: false,
        }
    }

    pub fn mock(mut self, mock: bool) -> Self {
        self.mock = mock;
        self
    }
}

/// A struct representing the close command.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CloseFrame<'t> {
    /// The reason as text string.
    pub reason: Cow<'t, str>,
}

/// Handlers for the RPC stream events.
pub trait ConnectionHandler: Send + 'static {
    /// Called when a connection to the Relay is established.
    fn connected(&mut self) {}

    /// Called when the Relay connection is closed.
    fn disconnected(&mut self, _frame: Option<CloseFrame<'static>>) {}

    /// Called when a message is received from the Relay.
    fn message_received(&mut self, message: Message);

    /// Called when an inbound error occurs, such as data deserialization
    /// failure, or an unknown response message ID.
    fn inbound_error(&mut self, _error: ClientError) {}

    /// Called when an outbound error occurs, i.e. failed to write to the
    /// websocket stream.
    fn outbound_error(&mut self, _error: ClientError) {}
}

#[derive(Clone)]
pub struct Client {
    #[cfg(not(feature = "mock"))]
    wc: WcClient,
    #[cfg(feature = "mock")]
    wc: mock::Mocker,
}
struct WrapperHandler<T: ConnectionHandler> {
    handler: T,
}
impl<T: ConnectionHandler> WrapperHandler<T> {
    fn new(handler: T) -> Self {
        Self { handler }
    }
}

impl<T: ConnectionHandler> WcHandler for WrapperHandler<T> {
    fn connected(&mut self) {
        self.handler.connected()
    }

    fn disconnected(
        &mut self,
        _frame: Option<walletconnect_sdk::client::websocket::CloseFrame<'static>>,
    ) {
        self.handler.disconnected(None);
    }

    fn message_received(&mut self, message: PublishedMessage) {
        self.handler.message_received(message.into());
    }

    fn inbound_error(&mut self, err: walletconnect_sdk::client::error::ClientError) {
        self.handler.inbound_error(err.into());
    }

    fn outbound_error(&mut self, err: walletconnect_sdk::client::error::ClientError) {
        self.handler.outbound_error(err.into());
    }
}

#[cfg(not(feature = "mock"))]
impl Client {
    pub fn new(handler: impl ConnectionHandler) -> Self {
        let wrapper = WrapperHandler::new(handler);
        let wc = WcClient::new(wrapper);
        Self { wc }
    }
}

#[cfg(feature = "mock")]
impl Client {
    pub fn mock(handler: impl ConnectionHandler) -> Self {
        let wc = mock::MOCK_FACTORY.create(handler);
        Self { wc }
    }
}

impl Client {
    pub async fn publish(
        &self,
        topic: Topic,
        message: impl Into<Arc<str>>,
        tag: u32,
        ttl: Duration,
        prompt: bool,
    ) -> Result<()> {
        self.wc.publish(topic, message, tag, ttl, prompt).await?;
        Ok(())
    }

    pub async fn subscribe(&self, topic: Topic) -> Result<SubscriptionId> {
        let id = self.wc.subscribe(topic).await?;
        Ok(id)
    }

    pub async fn unsubscribe(&self, topic: Topic) -> Result<()> {
        self.wc.unsubscribe(topic).await?;
        Ok(())
    }

    #[cfg(not(feature = "mock"))]
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

    #[cfg(feature = "mock")]
    pub async fn connect(&self, opts: &ConnectionOptions) -> Result<()> {
        self.wc.connect().await
    }

    pub async fn disconnect(&self) -> Result<()> {
        self.wc.disconnect().await.ok();
        Ok(())
    }
}
