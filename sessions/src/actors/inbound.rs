use std::future::Future;
use dashmap::DashMap;
use serde_json::Value;
use tokio::sync::oneshot;
use tracing::error;
use xtra::{Context, Handler};
use crate::domain::{MessageId, Topic};
use crate::relay::MessageIdGenerator;
use crate::rpc::{RequestParams, Response, ResponseParams};

#[derive(Default, xtra::Actor)]
pub(crate) struct InboundResponseActor {
  pending: DashMap<MessageId, oneshot::Sender<crate::Result<Value>>>,
  generator: MessageIdGenerator,
}

pub(crate) struct AddRequest;

impl Handler<AddRequest> for InboundResponseActor {
  type Return = (MessageId, oneshot::Receiver<crate::Result<Value>>);

  async fn handle(&mut self, _message: AddRequest, _ctx: &mut Context<Self>) -> Self::Return {
    let id = self.generator.next();
    let (tx, rx) = oneshot::channel::<crate::Result<Value>>();
    self.pending.insert(id.clone(), tx);
    (id, rx)
  }
}

impl InboundResponseActor {
  fn add(&self) -> (MessageId,oneshot::Receiver<crate::Result<Value>>) {
    let id = self.generator.next();
    let (tx, rx) = oneshot::channel::<crate::Result<Value>>();
    self.pending.insert(id.clone(), tx);
    (id, rx)
  }
}

impl Handler<Response> for InboundResponseActor {
  type Return = ();

  async fn handle(&mut self, message: Response, _ctx: &mut Context<Self>) -> Self::Return {
    if let Some((_, tx)) = self.pending.remove(&message.id) {
      let res: crate::Result<Value> = match message.params {
        ResponseParams::Success(v) => Ok(v),
        ResponseParams::Err(v) => Err(crate::Error::RpcError(v)),
      };
      let _ = tx.send(res);
      return;
    }
    error!("id [{}] not found for message {:#?}", message.id, message.params)
  }
}
