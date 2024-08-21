use std::future::Future;
use dashmap::DashMap;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::oneshot;
use tracing::{error, info};
use xtra::prelude::*;
use crate::crypto::CipherError;
use crate::domain::MessageId;
use crate::Message;
use crate::relay::MessageIdGenerator;
use crate::rpc::{Response, ResponseParams};
use crate::Result;

#[derive(Default, xtra::Actor)]
pub(crate) struct ResponseActor {
  pending: DashMap<MessageId, oneshot::Sender<Result<Value>>>,
  generator: MessageIdGenerator,
}

#[derive(Default, xtra::Actor)]
pub(crate) struct RequestActor {

}

pub(crate) struct AddRequest;

impl Handler<AddRequest> for ResponseActor {
  type Return = (MessageId, oneshot::Receiver<Result<Value>>);

  async fn handle(&mut self, _message: AddRequest, _ctx: &mut Context<Self>) -> Self::Return {
    let id = self.generator.next();
    let (tx, rx) = oneshot::channel::<Result<Value>>();
    self.pending.insert(id.clone(), tx);
    (id, rx)
  }
}

impl Handler<Response> for ResponseActor {
  type Return = ();

  async fn handle(&mut self, message: Response, _ctx: &mut Context<Self>) -> Self::Return {
    if let Some((_, tx)) = self.pending.remove(&message.id) {
      let res: Result<Value> = match message.params {
        ResponseParams::Success(v) => Ok(v),
        ResponseParams::Err(v) => Err(crate::Error::RpcError(v)),
      };
      let _ = tx.send(res);
      return;
    }
    error!("id [{}] not found for message {:#?}", message.id, message.params)
  }
}


#[cfg(test)]
mod test {
  use super::*;
  use xtra::prelude::*;
  use crate::rpc::{ResponseParamsSuccess, SessionProposeResponse};

  #[tokio::test]
  async fn test_payload_response() -> anyhow::Result<()> {
    let addr = xtra::spawn_tokio(ResponseActor::default(), Mailbox::unbounded());
    let (id, rx) = addr.send(AddRequest).await?;
    let addr_resp = addr.clone();
    tokio::spawn(async move {
      let v = serde_json::to_value(ResponseParamsSuccess::SessionPropose(SessionProposeResponse{
        responder_public_key: "blah".to_string(),
        relay: Default::default(),
      })).unwrap();
      let resp = Response::new(id, ResponseParams::Success(v));
      addr_resp.send(resp).await
    });
    let resp: Response = serde_json::from_value(rx.await?.unwrap())?;
    Ok(())
  }
}