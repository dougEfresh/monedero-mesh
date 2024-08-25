mod cipher;
mod inbound;
mod request;
mod session;
mod socket;
mod transport;

use crate::actors::cipher::CipherActor;
use crate::actors::request::RegisterTopicManager;
use crate::actors::session::SessionRequestHandlerActor;
pub(crate) use crate::actors::socket::SocketActor;
use crate::domain::Topic;
use crate::relay::Client;
use crate::rpc::{Proposer, RequestParams, SessionProposeResponse, SessionSettleRequest};
use crate::session::ClientSession;
use crate::transport::{SessionTransport, TopicTransport};
use crate::{Cipher, Pairing, PairingManager};
use crate::{Dapp, Result, Wallet};
pub(crate) use inbound::{AddRequest, InboundResponseActor};
pub(crate) use request::RequestHandlerActor;
use tracing::debug;
pub(crate) use transport::TransportActor;
use xtra::{Address, Mailbox};

#[derive(Clone)]
pub struct Actors {
    inbound_response_actor: Address<InboundResponseActor>,
    request_actor: Address<RequestHandlerActor>,
    transport_actor: Address<TransportActor>,
    socket_actor: Address<SocketActor>,
    cipher_actor: Address<CipherActor>,
    session_actor: Address<SessionRequestHandlerActor>,
    cipher: Cipher,
}

pub(crate) struct ClearPairing;
pub(crate) struct Subscribe(pub Topic);
pub(crate) struct RegisterDapp(pub Topic, pub Dapp);
pub(crate) struct RegisterWallet(pub Topic, pub Wallet);
pub struct RegisteredManagers;
pub(crate) struct SendRequest(pub(crate) Topic, pub(crate) RequestParams);
pub(crate) struct SessionSettled(pub Topic, pub SessionSettleRequest);
pub(crate) struct SessionPing;

impl Actors {
    pub(crate) async fn register_settlement(
        &self,
        transport: TopicTransport,
        settlement: SessionSettled,
    ) -> Result<ClientSession> {
        let session_transport = SessionTransport {
            topic: settlement.0.clone(),
            transport,
        };
        let client_session = ClientSession::new(session_transport, settlement.1.namespaces.clone());
        self.session_actor.send(client_session.clone()).await?;
        self.cipher_actor.send(settlement).await??;
        Ok(client_session)
    }

    pub async fn registered_managers(&self) -> Result<usize> {
        Ok(self.request_actor.send(RegisteredManagers).await?)
    }

    pub async fn register_wallet_pairing(&self, wallet: Wallet, pairing: Pairing) -> Result<()> {
        debug!("registering wallet to topic {}", pairing.topic);
        self.cipher_actor.send(pairing.clone()).await??;
        self.request_actor
            .send(RegisterWallet(pairing.topic.clone(), wallet))
            .await?;
        self.transport_actor
            .send(Subscribe(pairing.topic))
            .await??;

        Ok(())
    }

    pub async fn register_dapp_pk(&self, wallet: Wallet, proposer: Proposer) -> Result<Topic> {
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

    pub async fn register_wallet_pk(
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

    pub async fn register_mgr(&self, topic: Topic, mgr: PairingManager) -> Result<()> {
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
        let socket_actor = xtra::spawn_tokio(SocketActor::default(), Mailbox::unbounded());
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
        let cipher_actor =
            xtra::spawn_tokio(CipherActor::new(cipher.clone()), Mailbox::unbounded());
        Ok(Self {
            inbound_response_actor,
            request_actor,
            transport_actor,
            socket_actor,
            cipher_actor,
            session_actor,
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
