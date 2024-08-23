mod socket;
mod inbound;
mod request;
mod transport;

use std::future::Future;
use serde::de::DeserializeOwned;
use xtra::{Address, Mailbox};
use crate::rpc::{Response, ResponseParams};
pub(crate) use socket::SocketActors;
pub(crate) use request::{RequestHandlerActor, RegisteredManagers};
pub(crate) use inbound::{InboundResponseActor, AddRequest};
pub(crate) use transport::{TransportActor, SendRequest};
use crate::{Cipher, PairingManager, SocketHandler};
use crate::actors::request::RegisterTopicManager;
use crate::domain::Topic;
use crate::relay::Client;
use crate::Result;

#[derive(Clone)]
pub(crate) struct Actors {
  inbound_response_actor: Address<InboundResponseActor>,
  request_actor: Address<RequestHandlerActor>,
  transport_actor: Address<TransportActor>,
  socket_actors: Address<SocketActors>,
  cipher: Cipher
}

impl Actors {
  pub(crate) async fn register_mgr(&self, topic: Topic, mgr: PairingManager) -> Result<()> {
    Ok(self.request_actor.send(RegisterTopicManager(topic, mgr)).await?)
  }
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
  pub(crate) async fn init(cipher: Cipher) -> crate::Result<Self> {
    let inbound_response_actor = xtra::spawn_tokio(InboundResponseActor::default(), Mailbox::unbounded());
    let socket_actors = xtra::spawn_tokio(SocketActors::default(), Mailbox::unbounded());
    socket_actors.send(socket::Subscribe(socket::SocketLogActor::default())).await?;
    let transport_actor = xtra::spawn_tokio(TransportActor::new(cipher.clone(), inbound_response_actor.clone()), Mailbox::unbounded());
    let request_actor = xtra::spawn_tokio(RequestHandlerActor::new(transport_actor.clone()), Mailbox::unbounded());
    Ok(Self {
      inbound_response_actor,
      request_actor,
      transport_actor,
      socket_actors,
      cipher
    })
  }
}

impl Actors {

  pub(crate) fn cipher(&self) -> Cipher {
    self.cipher.clone()
  }

  pub(crate) fn response(&self) -> Address<InboundResponseActor> {
    self.inbound_response_actor.clone()
  }

  pub(crate) fn request(&self) -> Address<RequestHandlerActor> {
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