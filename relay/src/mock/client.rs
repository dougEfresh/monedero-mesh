use {
    super::{MockRelay, PendingMessages, WsPublishedMessage, WsSender},
    crate::Topic,
    dashmap::DashSet,
    reown_relay_client::MessageIdGenerator,
    reown_relay_rpc::{
        domain::{MessageId, SubscriptionId},
        rpc::{Params, Payload, Publish},
    },
    std::{
        fmt::{Debug, Display},
        hash::{Hash, Hasher},
        sync::Arc,
        time::Duration,
    },
    tokio::sync::broadcast::Receiver,
    tracing::{debug, warn, Level},
};

#[allow(dead_code)]
#[derive(Clone)]
pub struct WsClient {
    pub id: u16,
    topics: Arc<DashSet<Topic>>,
    ws_sender: WsSender,
    generator: MessageIdGenerator,
    pending: PendingMessages,
    // sent: SentMessages,
}

impl Hash for WsClient {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for WsClient {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl Eq for WsClient {}

impl Display for WsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_common())
    }
}
impl Debug for WsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_common())
    }
}

impl WsClient {
    fn fmt_common(&self) -> String {
        format!("[wsclient-{}]({})", self.id, self.topics.len())
    }

    pub fn new(relay: &MockRelay, id: u16, ws_sender: WsSender) -> Self {
        let me = Self {
            id,
            ws_sender,
            topics: Arc::new(DashSet::new()),
            generator: relay.generator.clone(),
            pending: relay.pending.clone(),
        };
        let listener = me.clone();
        tokio::spawn(listener.handle_message(relay.tx.subscribe()));
        me
    }

    async fn handle_message(self, mut rx: Receiver<WsPublishedMessage>) {
        while let Ok(published_message) = rx.recv().await {
            if published_message.close {
                if published_message.client_id == self.id {
                    debug!("{self} connection was closed");
                    return;
                }
                continue;
            }
            let id = published_message.payload.id();
            if published_message.client_id == self.id {
                self.handle_own_message(id, published_message);
                continue;
            }
            self.handle_published_message(id, published_message);
        }
    }

    #[tracing::instrument(level = Level::DEBUG, skip(published_message))]
    fn handle_published_message(&self, id: MessageId, published_message: WsPublishedMessage) {
        match &published_message.payload {
            Payload::Request(ref req) => {
                if let Params::Publish(ref p) = req.params {
                    if !self.topics.contains(&p.topic) {
                        warn!(
                            "{self} got a message but I am not subscribed to this topic {}",
                            p.topic
                        );
                        return;
                    }
                    self.pending.remove(p);
                    debug!(
                        "forwarding request from client_id:{} with",
                        published_message.client_id,
                    );
                    self.send_message(vec![p.clone()]);
                } else {
                    debug!("not handling message");
                }
            }
            Payload::Response(res) => debug!("not handling response payload {:?}", res),
        };
    }

    fn send_message(&self, messages: Vec<Publish>) {
        for p in messages {
            let forward_id = self.generator.next();
            let now = chrono::Utc::now().timestamp();
            let subscription_id = SubscriptionId::from(p.topic.as_ref());
            let sub_req = p.as_subscription_request(forward_id, subscription_id, now);
            let forward_payload = Payload::Request(sub_req);
            tokio::spawn(MockRelay::forward(
                forward_payload,
                self.ws_sender.clone(),
                Duration::from_millis(50),
            ));
        }
    }

    fn check_pending(&self, topic: &Topic) {
        let to_send: Vec<Publish> = self
            .pending
            .iter()
            .filter(|m| m.topic == *topic)
            .map(|m| m.clone())
            .collect();
        debug!("found {} to send", to_send.len());
        for p in &to_send {
            self.pending.remove(p);
        }
        self.send_message(to_send);
    }

    #[tracing::instrument(level = Level::DEBUG)]
    fn handle_own_message(&self, id: MessageId, published_message: WsPublishedMessage) {
        debug!("handle my own message");
        match published_message.payload {
            Payload::Request(ref req) => match &req.params {
                Params::Subscribe(s) => {
                    let sub_id = SubscriptionId::from(s.topic.as_ref());
                    debug!("subscribe request to subId:{} {}", sub_id, s.topic);
                    self.topics.insert(s.topic.clone());
                    tokio::spawn(MockRelay::handle_ack(id, self.ws_sender.clone(), sub_id));
                    self.check_pending(&s.topic);
                }
                Params::BatchSubscribe(b) => {
                    debug!("batch sub");
                    let mut ids: Vec<SubscriptionId> = Vec::with_capacity(b.topics.len());
                    for t in &b.topics {
                        ids.push(SubscriptionId::from(t.as_ref()));
                        self.topics.insert(t.clone());
                    }
                    tokio::spawn(MockRelay::handle_ack(id, self.ws_sender.clone(), ids));
                    for t in &b.topics {
                        self.check_pending(t);
                    }
                }
                Params::Unsubscribe(s) => {
                    tokio::spawn(MockRelay::handle_ack(id, self.ws_sender.clone(), true));
                    self.topics.remove(&s.topic);
                }
                Params::Publish(p) => {
                    debug!("responding to my own published message");
                    self.pending.insert(p.clone());
                    tokio::spawn(MockRelay::handle_ack(id, self.ws_sender.clone(), true));
                }
                _ => {}
            },
            Payload::Response(_) => {}
        };
    }
}
