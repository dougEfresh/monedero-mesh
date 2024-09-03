mod inbound;
mod request;
mod session;
mod socket;
mod transport;

use std::ops::Deref;
use crate::actors::request::RegisterTopicManager;
use crate::actors::session::SessionRequestHandlerActor;
pub(crate) use crate::actors::socket::SocketActor;
use crate::domain::Topic;
use crate::rpc::{Proposer, RequestParams, SessionProposeResponse, SessionSettleRequest};
use crate::session::ClientSession;
use crate::transport::{SessionTransport, TopicTransport};
use crate::{Cipher, NoopSessionDeleteHandler, Pairing, PairingManager, PairingTopic, SessionTopic, SubscriptionId};
use crate::{Dapp, Result, Wallet};
pub(crate) use inbound::{AddRequest, InboundResponseActor};
pub(crate) use request::RequestHandlerActor;
use walletconnect_relay::Client;

use tracing::{debug, info, warn};
pub(crate) use transport::TransportActor;
use xtra::{Address, Mailbox};

#[derive(Clone)]
pub struct Actors {
    inbound_response_actor: Address<InboundResponseActor>,
    request_actor: Address<RequestHandlerActor>,
    transport_actor: Address<TransportActor>,
    socket_actor: Address<SocketActor>,
    session_actor: Address<SessionRequestHandlerActor>,
    cipher: Cipher,
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
pub(crate) struct DeleteSession(pub Topic);

impl Deref for SessionSettled {
    type Target = SessionSettleRequest;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

pub(crate) enum RegisterComponent {
    WalletDappPublicKey(Wallet, Proposer),
    WalletPairTopic(Wallet),
    Dapp(Dapp, SessionProposeResponse),
    DappRestore(Dapp, SessionTopic),
    None,
}
pub(crate) struct RegisterPairing {
    pub pairing: Pairing,
    pub mgr: PairingManager,
    pub component: RegisterComponent,
}
impl RegisterPairing {
    pub(crate) fn has_existing_topic(&self) -> bool {
        match self.mgr.topic() {
            None => false,
            Some(t) => t == self.pairing.topic
        }
    }
}

impl Actors {

    pub(crate) async fn reset(&self) {
        if let Err(e) = self.request_actor.send(ClearPairing).await {
            warn!("failed to clean up request actor: {e}");
        }
    }

    pub(crate) async fn register_settlement(
        &self,
        client_session: ClientSession,
    ) -> Result<()> {
        self.session_actor.send(client_session.clone()).await?;
        Ok(())
        //self.cipher_actor.send(settlement).await?
    }

    pub async fn registered_managers(&self) -> Result<usize> {
        Ok(self.request_actor.send(RegisteredManagers).await?)
    }

    pub async fn register_wallet_pairing(&self, wallet: Wallet, pairing: Pairing) -> Result<()> {
        debug!("registering wallet to topic {}", pairing.topic);
        self.request_actor
            .send(RegisterWallet(pairing.topic.clone(), wallet))
            .await?;
        Ok(())
    }

    pub(crate) async fn register_pairing(
        &self,
        register: RegisterPairing,
    ) -> Result<Option<SessionTopic>> {
        let pairing_topic = register.pairing.topic.clone();
        let sub_id = self.register_manager(register.mgr.clone(), register.pairing.topic.clone()).await?;
        info!("Subscribed to pairing topic {pairing_topic} sub id: {sub_id}");
        if !register.has_existing_topic() {
            self.cipher.set_pairing(Some(register.pairing.clone()))?;
        }

        match register.component {
            RegisterComponent::WalletPairTopic(wallet) => {
                self.register_wallet_pairing(wallet.clone(), register.pairing.clone())
                    .await?;
                Ok(None)
            }
            RegisterComponent::WalletDappPublicKey(wallet, proposer) => {
                info!("registering wallet");
                Ok(Some(self.register_dapp_pk(wallet, proposer).await?))
            }
            RegisterComponent::Dapp(dapp, settlement) => {
                Ok(Some(self.register_wallet_pk(dapp, settlement).await?))
            }
            RegisterComponent::DappRestore(dapp, session_topic) => {
                info!("restoring session for {}", session_topic);
                self.request_actor
                  .send(RegisterDapp(session_topic.clone(), dapp))
                  .await?;
                self.transport_actor
                  .send(Subscribe(session_topic.clone()))
                  .await??;
                Ok(Some(session_topic))
            }
            RegisterComponent::None => Ok(None),
        }
    }

    async fn register_manager(&self, mgr: PairingManager, topic: PairingTopic) -> Result<SubscriptionId> {
        self.request_actor
          .send(RegisterTopicManager(
              topic.clone(),
              mgr,
          ))
          .await?;
        self
          .transport_actor
          .send(Subscribe(topic))
          .await?
    }

    async fn register_dapp_pk(&self, wallet: Wallet, proposer: Proposer) -> Result<Topic> {
        let (session_topic, _) = self.cipher.create_common_topic(proposer.public_key)?;
        self.request_actor
            .send(RegisterWallet(session_topic.clone(), wallet))
            .await?;
        // TODO: Do I need the subscriptionId?
        let id = self
            .transport_actor
            .send(Subscribe(session_topic.clone()))
            .await??;
        tracing::info!("subscribed to topic {session_topic:#?} with id {id:#?}");
        Ok(session_topic)
    }

    async fn register_wallet_pk(
        &self,
        dapp: Dapp,
        controller: SessionProposeResponse,
    ) -> Result<Topic> {
        let (session_topic, _) = self.cipher.create_common_topic(controller.responder_public_key)?;
        self.request_actor
            .send(RegisterDapp(session_topic.clone(), dapp))
            .await?;
        // TODO: Do I need the subscriptionId?
        self.transport_actor
            .send(Subscribe(session_topic.clone()))
            .await??;
        Ok(session_topic)
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
        Ok(Self {
            inbound_response_actor,
            request_actor,
            transport_actor,
            socket_actor,
            session_actor,
            cipher,
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

    pub(crate) fn sockets(&self) -> Address<SocketActor> {
        self.socket_actor.clone()
    }
}
