use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use dashmap::DashMap;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::oneshot;
use tracing::info;
use xtra::{Address, Context, Handler};
use crate::actors::{AddRequest, InboundResponseActor, RequestResponderActor};
use crate::Cipher;
use crate::domain::{MessageId, Topic};
use crate::relay::{Client, MessageIdGenerator};
use crate::rpc::{RelayProtocolMetadata, Request, RequestParams, RpcResponse};
use crate::Result;

#[derive(Clone, xtra::Actor)]
pub(crate) struct TransportActor {
  cipher: Cipher,
  relay: Option<Client>,
  inbound_response_actor: Address<InboundResponseActor>
}

pub(crate) struct SendRequest(pub(crate) Topic, pub(crate) RequestParams);

impl TransportActor {
  pub(crate) fn new(cipher: Cipher, inbound_response_actor: Address<InboundResponseActor>) -> Self {
    Self {
      cipher,
      inbound_response_actor,
      relay: None,
    }
  }
}

impl Handler<Client> for TransportActor {
  type Return = ();

  async fn handle(&mut self, message: Client, ctx: &mut Context<Self>) ->Self::Return {
    self.relay = Some(message);
  }
}

impl Handler<RpcResponse> for TransportActor {
  type Return = ();

  async fn handle(&mut self, message: RpcResponse, ctx: &mut Context<Self>) -> Self::Return {
    tracing::debug!("sending response to id:{} on topic {} ", message.payload.id, message.topic);
  }
}

impl Handler<SendRequest> for TransportActor {
  type Return = Result<(MessageId, Duration, oneshot::Receiver<Result<Value>>)>;

  async fn handle(&mut self, message: SendRequest, _ctx: &mut Context<Self>) -> Self::Return {
    let relay = self.relay.as_ref().ok_or(crate::Error::NoClient)?;
    let (id, rx) = self.inbound_response_actor.send(AddRequest).await?;

    let topic = message.0;
    let params = message.1;
    let irn_metadata = params.irn_metadata();
    let request = Request::new(id, params);
    info!("Sending request topic={topic}");
    let encrypted = self.cipher.encode(&topic, &request)?;
    let ttl = Duration::from_secs(irn_metadata.ttl);
    relay.publish(topic,
        Arc::from(encrypted),
        irn_metadata.tag,
        ttl.clone(),
        irn_metadata.prompt,
      ).await?;
    Ok((id, ttl, rx))
  }

}




