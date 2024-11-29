pub use walletconnect_sdk::{
    client::MessageIdGenerator,
    rpc::{
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
    serde::{Deserialize, Serialize},
    std::{
        borrow::Cow,
        fmt::{Debug, Display, Formatter},
        sync::Arc,
        time::Duration,
    },
    walletconnect_sdk::{
        client::{websocket::PublishedMessage, Authorization},
        rpc::auth::ed25519_dalek::SigningKey,
    },
};

pub const RELAY_ADDRESS: &str = "wss://relay.walletconnect.com";
mod error;

#[cfg(not(feature = "mock"))]
mod client;
#[cfg(not(feature = "mock"))]
pub use client::Client;

pub type PairingTopic = Topic;
pub type SessionTopic = Topic;
pub const RELAY_PROTOCOL: &str = "irn";

#[cfg(feature = "mock")]
mod mock;
pub use error::ClientError;
#[cfg(feature = "mock")]
pub use mock::*;
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

    #[cfg(feature = "mock")]
    /// Create a mock connection and bind the connections by this pairId
    /// Only used for tests
    pub conn_pair: mock::ConnectionPair,
}

impl ConnectionOptions {
    #[cfg(feature = "mock")]
    #[must_use]
    pub fn new(
        project_id: ProjectId,
        serialized: SerializedAuthToken,
        conn_pair: ConnectionPair,
    ) -> Self {
        Self {
            address: RELAY_ADDRESS.into(),
            project_id,
            auth: Authorization::Query(serialized),
            origin: None,
            user_agent: None,
            conn_pair,
        }
    }

    #[cfg(not(feature = "mock"))]
    pub fn new(project_id: ProjectId, serialized: SerializedAuthToken) -> Self {
        Self {
            address: RELAY_ADDRESS.into(),
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

/// # Panics
///
/// Will panic key is invalid
#[allow(clippy::unwrap_used)]
pub fn auth_token(url: impl Into<String>) -> SerializedAuthToken {
    let key = SigningKey::generate(&mut rand::thread_rng());
    AuthToken::new(url)
        .aud(RELAY_ADDRESS)
        .ttl(Duration::from_secs(60 * 60))
        .as_jwt(&key)
        .unwrap()
}
