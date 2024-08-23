use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use dashmap::DashMap;
use serde_json::json;
use crate::transport::{PairingRpcEvent, RpcRecv};
use xtra::prelude::*;
use crate::domain::Topic;
use crate::{rpc, Cipher, PairingManager};
use crate::actors::TransportActor;
use crate::relay::Client;
use crate::rpc::{ErrorParams, PairPingRequest, Request, RequestParams, Response, ResponseParams, ResponseParamsError, RpcRequest, RpcResponse};


#[derive(xtra::Actor)]
pub(crate) struct RequestHandlerActor {
  pair_managers: Arc<DashMap<Topic, Address<PairingManager>>>,
  responder: Address<TransportActor>,
}

pub(crate) struct RegisteredManagers;

impl Handler<RegisteredManagers> for RequestHandlerActor {
  type Return = usize;

  async fn handle(&mut self, _message: RegisteredManagers, _ctx: &mut Context<Self>) -> Self::Return {
    self.pair_managers.len()
  }
}

pub(crate) struct RegisterTopicManager(pub(crate) Topic, pub(crate) PairingManager);

impl Handler<RegisterTopicManager> for RequestHandlerActor {
  type Return = ();

  async fn handle(&mut self, message: RegisterTopicManager, _ctx: &mut Context<Self>) -> Self::Return {
    tracing::info!("registering mgr for topic {}", message.0);
    let addr = xtra::spawn_tokio(message.1, Mailbox::unbounded());
    self.pair_managers.insert(message.0, addr);
  }
}

impl Handler<Client> for RequestHandlerActor {
  type Return = crate::Result<()>;

  async fn handle(&mut self, message: Client, ctx: &mut Context<Self>) -> Self::Return {
    self.send_client(message).await
  }
}

impl RequestHandlerActor {
  pub(crate) fn new(responder: Address<TransportActor>) -> Self {
    Self {
      pair_managers: Arc::new(DashMap::new()),
      responder,
    }
  }

  pub(crate) async fn send_client(&self, relay: Client) -> crate::Result<()> {
    Ok(self.responder.send(relay).await?)
  }
}

impl Handler<RpcRequest> for RequestHandlerActor {
  type Return = ();

  async fn handle(&mut self, message: RpcRequest, _ctx: &mut Context<Self>) ->  Self::Return {
    let id = message.payload.id.clone();
    let topic = message.topic.clone();
    let responder = self.responder.clone();
    match &message.payload.params {
      RequestParams::PairDelete(_)  => {}
      RequestParams::PairExtend(_)  => {}
      RequestParams::PairPing(_) => {
        if let Some(mgr) = self.pair_managers.get(&message.topic) {
          let rpc_response: RpcResponse = match mgr.send(PairPingRequest{}).await {
            Ok(response) => {
              RpcResponse::into_response(id, topic, response)
            }
            Err(_) => {
              let r = ResponseParamsError::PairPing(ErrorParams{code: Some(1), message: String::from("unknown error")});
              let params: ResponseParams = r.try_into().unwrap();
              RpcResponse::into_response(id, topic, params)
            }
          };
          tokio::spawn(async move {
            let _ = responder.send(rpc_response).await;
          });
        } else {
          tracing::warn!("topic {topic} has no pairing manager!");
        }
      }
      RequestParams::SessionPropose(args) => {

      }
      RequestParams::SessionSettle(_) => {}
      RequestParams::SessionUpdate(_) => {}
      RequestParams::SessionExtend(_) => {}
      RequestParams::SessionRequest(_) => {}
      RequestParams::SessionEvent(_) => {}
      RequestParams::SessionDelete(_) => {}
      RequestParams::SessionPing(_) => {}
    }
  }
}

#[cfg(test)]
mod test {

  #[tokio::test]
  async fn test_request_actor() -> anyhow::Result<()> {

    Ok(())
  }
}