mod inbound;
mod pair_manager_requests;
mod proposal;
mod request;
mod session;
mod session_handlers;
mod transport;

pub(crate) use {
    crate::actors::session::SessionRequestHandlerActor,
    inbound::InboundResponseActor,
    request::RequestHandlerActor,
    transport::TransportActor,
};
use {
    crate::{actors::proposal::ProposalActor, domain::Topic, rpc::RequestParams, Cipher, Result},
    monedero_relay::Client,
    std::{
        fmt::{Debug, Display, Formatter},
        ops::Deref,
    },
    xtra::{Address, Mailbox},
};

#[derive(Clone)]
pub struct Actors {
    inbound_response_actor: Address<InboundResponseActor>,
    request_actor: Address<RequestHandlerActor>,
    transport_actor: Address<TransportActor>,
    session_actor: Address<SessionRequestHandlerActor>,
    proposal_actor: Address<ProposalActor>,
}

pub(crate) struct ClearPairing;
pub(crate) struct Unsubscribe(pub Topic);
pub(crate) struct SendRequest(pub(crate) Topic, pub(crate) RequestParams);
pub(crate) struct SessionPing;
pub(crate) struct AddRequest;
pub struct ClearSession(pub Topic);

impl Display for SendRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "topic={} request={}",
            crate::shorten_topic(&self.0),
            &self.1
        )
    }
}

/// Get number of sessions/pair managers are active
pub struct RegisteredComponents;

impl Actors {
    pub(crate) async fn register_client(&self, relay: Client) -> Result<()> {
        let _ = self.request_actor.send(relay).await?;
        Ok(())
    }
}

impl Actors {
    pub(crate) fn init(cipher: Cipher) -> Self {
        let inbound_response_actor =
            xtra::spawn_tokio(InboundResponseActor::default(), Mailbox::unbounded());
        let transport_actor = xtra::spawn_tokio(
            TransportActor::new(cipher.clone(), inbound_response_actor.clone()),
            Mailbox::unbounded(),
        );
        let session_actor = xtra::spawn_tokio(
            SessionRequestHandlerActor::new(transport_actor.clone(), cipher.clone()),
            Mailbox::unbounded(),
        );
        let proposal_actor = xtra::spawn_tokio(
            ProposalActor::new(transport_actor.clone()),
            Mailbox::unbounded(),
        );
        let request_actor = xtra::spawn_tokio(
            RequestHandlerActor::new(
                transport_actor.clone(),
                session_actor.clone(),
                proposal_actor.clone(),
            ),
            Mailbox::unbounded(),
        );

        Self {
            inbound_response_actor,
            request_actor,
            transport_actor,
            session_actor,
            proposal_actor,
        }
    }
}

impl Actors {
    pub fn response(&self) -> Address<InboundResponseActor> {
        self.inbound_response_actor.clone()
    }

    pub fn request(&self) -> Address<RequestHandlerActor> {
        self.request_actor.clone()
    }

    pub fn transport(&self) -> Address<TransportActor> {
        self.transport_actor.clone()
    }

    pub fn session(&self) -> Address<SessionRequestHandlerActor> {
        self.session_actor.clone()
    }

    pub fn proposal(&self) -> Address<ProposalActor> {
        self.proposal_actor.clone()
    }
}
