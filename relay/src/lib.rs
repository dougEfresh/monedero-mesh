pub use {
    reown_relay_client::MessageIdGenerator,
    reown_relay_rpc::{
        auth::*,
        domain::{
            ClientIdDecodingError,
            DecodedTopic,
            MessageId,
            ProjectId,
            SubscriptionId,
            Topic,
        },
        user_agent::*,
    },
};
use {
    reown_relay_client::{websocket::PublishedMessage, Authorization},
    reown_relay_rpc::auth::ed25519_dalek::SigningKey,
    serde::{Deserialize, Serialize},
    std::{
        borrow::Cow,
        fmt::{Debug, Display, Formatter},
        sync::Arc,
        time::Duration,
    },
};

pub const RELAY_ADDRESS: &str = "wss://relay.walletconnect.com";
pub(crate) const MOCK_RELAY_ADDRESS: &str = "127.0.0.1:4001";
pub const RELAY_PROTOCOL: &str = "irn";
pub const AUTH_URL: &str = "https://cartera-mesh.com";

mod client;
mod error;
#[cfg(not(target_family = "wasm"))]
mod mock;
pub use client::Client;
#[cfg(not(target_family = "wasm"))]
pub use mock::MockRelay;
pub type PairingTopic = Topic;
pub type SessionTopic = Topic;
pub use error::ClientError;
pub type Result<T> = std::result::Result<T, ClientError>;

pub fn shorten_topic(id: &Topic) -> String {
    let mut id = format!("{id}");
    if id.len() > 10 {
        id = String::from(&id[0..9]);
    }
    id
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub subscription_id: SubscriptionId,
    pub topic: Topic,
    pub message: Arc<str>,
    pub tag: u32,
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub received_at: chrono::DateTime<chrono::Utc>,
}

impl Debug for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "id={} topic:{} tag:{}", self.id, self.topic, self.tag)
    }
}

impl Default for Message {
    fn default() -> Self {
        let s = SubscriptionId::generate();
        let t = Topic::generate();
        Self {
            id: MessageIdGenerator::new().next(),
            subscription_id: s,
            topic: t,
            message: String::new().into(),
            tag: 1000,
            published_at: chrono::DateTime::default(),
            received_at: chrono::DateTime::default(),
        }
    }
}

impl Message {
    #[must_use]
    pub fn tag_name(&self) -> String {
        // TODO
        format!("{}", self.tag)
    }
}

impl From<PublishedMessage> for Message {
    fn from(value: PublishedMessage) -> Self {
        Self {
            id: value.message_id,
            subscription_id: value.subscription_id,
            topic: value.topic,
            message: value.message,
            tag: value.tag,
            published_at: value.published_at,
            received_at: value.received_at,
        }
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "id: {} subId: {} topic: {} tag: {}",
            self.id,
            self.subscription_id,
            self.topic,
            self.tag_name()
        )
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionOptions {
    /// The Relay websocket address. The default address is
    /// `wss://relay.walletconnect.com`.
    pub address: String,

    /// The project-specific secret key. Can be generated in the Cloud Dashboard
    /// at the following URL: <https://cloud.walletconnect.com/app>
    pub project_id: ProjectId,

    /// The authorization method and auth token to use.
    #[allow(dead_code)]
    pub(crate) auth: Authorization,

    /// Optional origin of the request. Subject to allow-list validation.
    pub origin: Option<String>,

    pub user_agent: Option<UserAgent>,
}

impl ConnectionOptions {
    pub fn mock(project_id: ProjectId, serialized: SerializedAuthToken) -> Self {
        Self::create(
            format!("ws://{MOCK_RELAY_ADDRESS}").as_str(),
            project_id,
            serialized,
        )
    }

    pub fn new(project_id: ProjectId, serialized: SerializedAuthToken) -> Self {
        Self::create(RELAY_ADDRESS, project_id, serialized)
    }

    fn create(address: &str, project_id: ProjectId, serialized: SerializedAuthToken) -> Self {
        Self {
            address: address.into(),
            project_id,
            auth: Authorization::Query(serialized),
            origin: None,
            user_agent: None,
        }
    }
}

/// A struct representing the close command.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CloseFrame<'t> {
    /// The reason as text string.
    pub reason: Cow<'t, str>,
}

pub struct NoopHandler;

impl ConnectionHandler for NoopHandler {
    fn message_received(&mut self, _message: Message) {}
}

pub struct LogHandler {
    handler: Box<dyn ConnectionHandler>,
}

impl LogHandler {
    pub fn new(handler: impl ConnectionHandler) -> Self {
        Self {
            handler: Box::new(handler),
        }
    }
}

impl ConnectionHandler for LogHandler {
    fn message_received(&mut self, message: Message) {
        self.handler.message_received(message);
    }
}

/// Handlers for the RPC events.
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

pub fn mock_connection_opts(project_id: &ProjectId) -> ConnectionOptions {
    let auth = auth_token(AUTH_URL);
    ConnectionOptions::mock(project_id.clone(), auth)
}

pub fn default_connection_opts(project_id: &ProjectId) -> ConnectionOptions {
    let auth = auth_token(AUTH_URL);
    ConnectionOptions::new(project_id.clone(), auth)
}

/// # Panics
///
/// Will panic when key is invalid
#[allow(clippy::unwrap_used)]
pub fn auth_token(url: impl Into<String>) -> SerializedAuthToken {
    let key = SigningKey::generate(&mut rand::thread_rng());
    AuthToken::new(url)
        .aud(RELAY_ADDRESS)
        .ttl(Duration::from_secs(60 * 60))
        .as_jwt(&key)
        .unwrap()
}
