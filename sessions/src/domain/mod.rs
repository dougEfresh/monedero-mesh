use crate::relay::MessageIdGenerator;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use walletconnect_sdk::client::websocket::PublishedMessage;
pub use walletconnect_sdk::rpc::domain::{MessageId, ProjectId, SubscriptionId, Topic};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub subscription_id: SubscriptionId,
    pub topic: Topic,
    pub message: Arc<str>,
    pub tag: u32,
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub received_at: chrono::DateTime<chrono::Utc>,
}

impl Default for Message {
    fn default() -> Self {
        let s = SubscriptionId::generate();
        let t = Topic::generate();
        Self {
            id: MessageIdGenerator::new().next(),
            subscription_id: s,
            topic: t,
            message: "blah".into(),
            tag: crate::rpc::TAG_SESSION_PROPOSE_REQUEST,
            published_at: Default::default(),
            received_at: Default::default(),
        }
    }
}

impl Message {
    pub fn tag_name(&self) -> String {
        //TODO
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
