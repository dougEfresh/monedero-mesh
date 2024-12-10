use {
    crate::{
        actors::{SendRequest, TransportActor, Unsubscribe},
        rpc::{RequestParams, ResponseParams},
        wait, Result,
    },
    monedero_domain::Topic,
    serde::de::DeserializeOwned,
    std::fmt::{Debug, Display, Formatter},
    xtra::Address,
};

#[derive(Clone)]
pub struct TopicTransport {
    transport_actor: Address<TransportActor>,
}

impl TopicTransport {
    pub(crate) async fn unsubscribe(&self, topic: Topic) -> Result<()> {
        self.transport_actor.send(Unsubscribe(topic)).await?
    }
}

impl TopicTransport {
    pub(crate) const fn new(transport_actor: Address<TransportActor>) -> Self {
        Self { transport_actor }
    }

    #[allow(clippy::cast_possible_truncation)]
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn publish_request<R: DeserializeOwned>(
        &self,
        topic: Topic,
        params: RequestParams,
    ) -> Result<R> {
        let (id, ttl, rx) = self
            .transport_actor
            .send(SendRequest(topic, params))
            .await??;

        if let Ok(result) = wait::wait_until((ttl.as_secs() * 1000) as u32, rx).await {
            return match result {
                Ok(response) => match response.params {
                    ResponseParams::Success(v) => Ok(serde_json::from_value(v)?),
                    ResponseParams::Err(v) => Err(crate::Error::RpcError(v)),
                },
                Err(_) => Err(crate::Error::ResponseChannelError(id)),
            };
        }
        Err(crate::Error::ResponseTimeout)
    }
}

#[derive(Clone)]
pub struct SessionTransport {
    pub(crate) topic: Topic,
    pub(crate) transport: TopicTransport,
}

impl Debug for SessionTransport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "topic={}", crate::shorten_topic(&self.topic))
    }
}

impl Display for SessionTransport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "topic={}", crate::shorten_topic(&self.topic))
    }
}

impl SessionTransport {
    pub async fn publish_request<R: DeserializeOwned>(&self, params: RequestParams) -> Result<R> {
        self.transport
            .publish_request(self.topic.clone(), params)
            .await
    }
}
