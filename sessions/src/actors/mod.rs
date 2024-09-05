mod inbound;
mod pair_manager_requests;
mod request;
mod session;
mod transport;

use crate::actors::session::SessionRequestHandlerActor;
use crate::domain::Topic;
use crate::rpc::{RequestParams, SessionSettleRequest};
use crate::{Cipher, PairingManager};
use crate::{Dapp, Result, Wallet};
pub(crate) use inbound::InboundResponseActor;
pub(crate) use request::RequestHandlerActor;
use std::ops::Deref;
use walletconnect_relay::Client;

pub(crate) use transport::TransportActor;
use xtra::{Address, Mailbox};

#[derive(Clone)]
pub struct Actors {
    inbound_response_actor: Address<InboundResponseActor>,
    request_actor: Address<RequestHandlerActor>,
    transport_actor: Address<TransportActor>,
    session_actor: Address<SessionRequestHandlerActor>,
}

pub(crate) struct ClearPairing;
pub(crate) struct Subscribe(pub Topic);
pub(crate) struct Unsubscribe(pub Topic);
pub(crate) struct RegisterDapp(pub Topic, pub Dapp);
pub(crate) struct RegisterWallet(pub Topic, pub Wallet);
pub struct RegisteredManagers;
pub(crate) struct SendRequest(pub(crate) Topic, pub(crate) RequestParams);
#[derive(Clone)]
pub(crate) struct SessionSettled(pub Topic, pub SessionSettleRequest);
pub(crate) struct SessionPing;
pub(crate) struct RegisterTopicManager(pub(crate) Topic, pub(crate) PairingManager);
pub(crate) struct AddRequest;

impl Deref for SessionSettled {
    type Target = SessionSettleRequest;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl Actors {
    pub(crate) async fn register_client(&self, relay: Client) -> Result<()> {
        let _ = self.request_actor.send(relay).await?;
        Ok(())
    }
}

impl Actors {
    pub(crate) fn init(cipher: Cipher) -> Result<Self> {
        let inbound_response_actor =
            xtra::spawn_tokio(InboundResponseActor::default(), Mailbox::unbounded());
        let transport_actor = xtra::spawn_tokio(
            TransportActor::new(cipher.clone(), inbound_response_actor.clone()),
            Mailbox::unbounded(),
        );
        let session_actor = xtra::spawn_tokio(
            SessionRequestHandlerActor::new(transport_actor.clone()),
            Mailbox::unbounded(),
        );
        let request_actor = xtra::spawn_tokio(
            RequestHandlerActor::new(transport_actor.clone(), session_actor.clone()),
            Mailbox::unbounded(),
        );
        Ok(Self {
            inbound_response_actor,
            request_actor,
            transport_actor,
            session_actor,
        })
    }
}

impl Actors {
    pub(crate) fn response(&self) -> Address<InboundResponseActor> {
        self.inbound_response_actor.clone()
    }

    pub(crate) fn request(&self) -> Address<RequestHandlerActor> {
        self.request_actor.clone()
    }

    pub(crate) fn transport(&self) -> Address<TransportActor> {
        self.transport_actor.clone()
    }

    pub(crate) fn session(&self) -> Address<SessionRequestHandlerActor> {
        self.session_actor.clone()
    }
}
