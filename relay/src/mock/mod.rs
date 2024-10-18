use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use std::time::Duration;

use crate::{ConnectionHandler, ConnectionOptions, Message, Result, SubscriptionId, Topic};

mod factory;
mod mocker;

type ConnectionPairId = Topic;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum ConnectionCategory {
    Dapp,
    Wallet,
}

#[derive(Clone, Debug)]
pub enum MockEvent {
    Open,
    Closed,
    Pending(Message),
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct ConnectionPair(pub ConnectionPairId, pub ConnectionCategory);

impl ConnectionPair {
    fn fmt_common(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.1 {
            ConnectionCategory::Dapp => {
                write!(f, "[dapp][{}]", crate::shorten_topic(&self.0))
            }
            ConnectionCategory::Wallet => {
                write!(f, "[wallet][{}]", crate::shorten_topic(&self.0))
            }
        }
    }
}

impl Display for ConnectionPair {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt_common(f)
    }
}

impl Debug for ConnectionPair {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt_common(f)
    }
}

#[derive(Clone)]
pub struct Client {
    wc: mocker::Mocker,
}

impl Debug for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.wc)
    }
}

impl Display for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.wc)
    }
}

impl Client {
    pub fn new(handler: impl ConnectionHandler, conn_pair: &ConnectionPair) -> Self {
        let wc = factory::MOCK_FACTORY.create(handler, conn_pair);
        Self { wc }
    }
}

impl Client {
    #[allow(clippy::missing_errors_doc)]
    pub async fn publish(
        &self,
        topic: Topic,
        message: impl Into<Arc<str>> + Send,
        tag: u32,
        ttl: Duration,
        prompt: bool,
    ) -> Result<()> {
        self.wc.publish(topic, message, tag, ttl, prompt).await
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn subscribe(&self, topic: Topic) -> Result<SubscriptionId> {
        self.wc.subscribe(topic).await
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn unsubscribe(&self, topic: Topic) -> Result<()> {
        self.wc.unsubscribe(topic).await
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn connect(&self, opts: &ConnectionOptions) -> Result<()> {
        self.wc.connect().await
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn disconnect(&self) -> Result<()> {
        self.wc.disconnect().await
    }

    pub async fn batch_subscribe(
        &self,
        topics: impl Into<Vec<Topic>>,
    ) -> Result<Vec<SubscriptionId>> {
        self.wc.batch_subscribe(topics).await
    }
}
