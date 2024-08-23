mod socket;
mod inbound;
mod request;
mod transport;

use std::future::Future;
use serde::de::DeserializeOwned;
use xtra::{Address, Mailbox};
use crate::rpc::{Response, ResponseParams};
pub(crate) use socket::SocketActors;
pub(crate) use request::RequestActor;
pub(crate) use inbound::{InboundResponseActor, AddRequest};
pub(crate) use request::RequestResponderActor;
pub(crate) use transport::{TransportActor, SendRequest};
use crate::{Cipher, SocketHandler};
use crate::relay::Client;

#[derive(Clone)]
pub(crate) struct Actors {
  inbound_response_actor: Address<InboundResponseActor>,
  request_actor: Address<RequestActor>,
  transport_actor: Address<TransportActor>,
  socket_actors: Address<SocketActors>,
}

impl Actors {
  pub(crate) async fn register_socket_handler<T: SocketHandler>(&self, handler: T) -> crate::Result<()> {
    Ok(self.socket_actors.send(socket::Subscribe(handler)).await?)
  }

  pub(crate) async fn register_client(&self, relay: Client) -> crate::Result<()> {
    let _ = self.request_actor.send(relay).await?;
    Ok(())
  }
}

impl Actors {
  pub(crate) fn init(cipher: Cipher) -> Self {
    let inbound_response_actor = xtra::spawn_tokio(InboundResponseActor::default(), Mailbox::unbounded());
    let socket_actors = xtra::spawn_tokio(SocketActors::default(), Mailbox::unbounded());
    let transport_actor = xtra::spawn_tokio(TransportActor::new(cipher, inbound_response_actor.clone()), Mailbox::unbounded());
    let request_actor = xtra::spawn_tokio(RequestActor::new(transport_actor.clone()), Mailbox::unbounded());
    Self {
      inbound_response_actor,
      request_actor,
      transport_actor,
      socket_actors,
    }
  }
}

impl Actors {
  pub(crate) fn response(&self) -> Address<InboundResponseActor> {
    self.inbound_response_actor.clone()
  }

  pub(crate) fn request(&self) -> Address<RequestActor> {
    self.request_actor.clone()
  }

  pub(crate) fn transport(&self) -> Address<TransportActor> {
    self.transport_actor.clone()
  }

  pub(crate) fn sockets(&self) -> Address<SocketActors> {
    self.socket_actors.clone()
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use xtra::prelude::*;
  use crate::actors::inbound::{AddRequest, InboundResponseActor};
  use crate::rpc::{ResponseParamsSuccess, SessionProposeResponse};

  #[tokio::test]
  async fn test_payload_response() -> anyhow::Result<()> {
    let addr = xtra::spawn_tokio(InboundResponseActor::default(), Mailbox::unbounded());
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