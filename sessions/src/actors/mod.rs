mod cipher;
mod inbound;
mod request;
mod socket;
mod transport;

use crate::actors::cipher::CipherActor;
use crate::actors::request::RegisterTopicManager;
pub(crate) use crate::actors::socket::SocketActor;
use crate::domain::Topic;
use crate::relay::Client;
use crate::rpc::{Proposer, SessionProposeResponse};
use crate::{Cipher, PairingManager, SocketHandler};
use crate::{Dapp, Result, Wallet};
pub(crate) use inbound::{AddRequest, InboundResponseActor};
pub(crate) use request::{RegisteredManagers, RequestHandlerActor};
use serde::de::DeserializeOwned;
use std::future::Future;
pub(crate) use transport::{SendRequest, TransportActor};
use xtra::{Address, Mailbox};

#[derive(Clone)]
pub(crate) struct Actors {
    inbound_response_actor: Address<InboundResponseActor>,
    request_actor: Address<RequestHandlerActor>,
    transport_actor: Address<TransportActor>,
    socket_actor: Address<SocketActor>,
    cipher_actor: Address<CipherActor>,
    cipher: Cipher,
}

pub(crate) struct ClearPairing;
pub(crate) struct Subscribe(pub Topic);
pub(crate) struct RegisterDapp(pub Topic, pub Dapp);
pub(crate) struct RegisterWallet(pub Topic, pub Wallet);

impl Actors {
    pub(crate) async fn register_dapp_pk(
        &self,
        wallet: Wallet,
        proposer: Proposer,
    ) -> Result<Topic> {
        let session_topic = self.cipher_actor.send(proposer).await??;
        self.request_actor
            .send(RegisterWallet(session_topic.clone(), wallet))
            .await?;
        // TODO: Do I need the subscriptionId?
        self.transport_actor
            .send(Subscribe(session_topic.clone()))
            .await??;
        Ok(session_topic)
    }

    pub(crate) async fn register_proposal_pk(
        &self,
        dapp: Dapp,
        controller: SessionProposeResponse,
    ) -> Result<Topic> {
        let session_topic = self.cipher_actor.send(controller).await??;
        self.request_actor
            .send(RegisterDapp(session_topic.clone(), dapp))
            .await?;
        // TODO: Do I need the subscriptionId?
        self.transport_actor
            .send(Subscribe(session_topic.clone()))
            .await??;
        Ok(session_topic)
    }

    pub(crate) async fn register_mgr(&self, topic: Topic, mgr: PairingManager) -> Result<()> {
        Ok(self
            .request_actor
            .send(RegisterTopicManager(topic, mgr))
            .await?)
    }
}

impl Actors {
    pub(crate) async fn register_socket_handler(
        &self,
        handler: Address<PairingManager>,
    ) -> Result<()> {
        Ok(self.socket_actor.send(handler).await?)
    }

    pub(crate) async fn register_client(&self, relay: Client) -> Result<()> {
        let _ = self.request_actor.send(relay).await?;
        Ok(())
    }
}

impl Actors {
    pub(crate) async fn init(cipher: Cipher) -> Result<Self> {
        let inbound_response_actor =
            xtra::spawn_tokio(InboundResponseActor::default(), Mailbox::unbounded());
        let socket_actors = xtra::spawn_tokio(SocketActor::default(), Mailbox::unbounded());
        let transport_actor = xtra::spawn_tokio(
            TransportActor::new(cipher.clone(), inbound_response_actor.clone()),
            Mailbox::unbounded(),
        );
        let request_actor = xtra::spawn_tokio(
            RequestHandlerActor::new(transport_actor.clone()),
            Mailbox::unbounded(),
        );
        let cipher_actor =
            xtra::spawn_tokio(CipherActor::new(cipher.clone()), Mailbox::unbounded());
        Ok(Self {
            inbound_response_actor,
            request_actor,
            transport_actor,
            socket_actor: socket_actors,
            cipher_actor,
            cipher,
        })
    }
}

impl Actors {
    pub(crate) fn cipher(&self) -> Cipher {
        self.cipher.clone()
    }
    pub(crate) fn cipher_actor(&self) -> Address<CipherActor> {
        self.cipher_actor.clone()
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

    pub(crate) fn sockets(&self) -> Address<SocketActor> {
        self.socket_actor.clone()
    }
}
