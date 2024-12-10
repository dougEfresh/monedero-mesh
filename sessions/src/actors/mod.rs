mod inbound;
mod pair_manager_requests;
mod proposal;
mod request;
mod session;
mod session_handlers;
mod transport;

pub use {
    crate::actors::session::SessionRequestHandlerActor, inbound::InboundResponseActor,
    request::RequestHandlerActor, transport::TransportActor,
};
use {
    crate::{actors::proposal::ProposalActor, rpc::RequestParams, Result},
    monedero_cipher::Cipher,
    monedero_domain::Topic,
    monedero_relay::Client,
    std::fmt::{Display, Formatter},
    xtra::{Actor, Address, Mailbox},
};

#[derive(Clone)]
pub struct Actors {
    inbound_response_actor: Address<InboundResponseActor>,
    request_actor: Address<RequestHandlerActor>,
    transport_actor: Address<TransportActor>,
    session_actor: Address<SessionRequestHandlerActor>,
    proposal_actor: Address<ProposalActor>,
}

pub struct ClearPairing;
pub struct Unsubscribe(pub Topic);
pub struct SendRequest(pub(crate) Topic, pub(crate) RequestParams);
pub struct SessionPing;
pub struct AddRequest;
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

pub fn actor_spawn<A>(actor: A) -> Address<A>
where
    A: Actor<Stop = ()>,
{
    #[cfg(not(target_arch = "wasm32"))]
    return xtra::spawn_tokio(actor, Mailbox::unbounded());
    #[cfg(target_arch = "wasm32")]
    return xtra::spawn_wasm_bindgen(actor, Mailbox::unbounded());
}

impl Actors {
    pub(crate) fn init(cipher: Cipher) -> Self {
        let inbound_response_actor = actor_spawn(InboundResponseActor::default());
        let transport_actor = actor_spawn(TransportActor::new(
            cipher.clone(),
            inbound_response_actor.clone(),
        ));
        let session_actor = actor_spawn(SessionRequestHandlerActor::new(
            transport_actor.clone(),
            cipher,
        ));
        let proposal_actor = actor_spawn(ProposalActor::new(transport_actor.clone()));
        let request_actor = actor_spawn(RequestHandlerActor::new(
            transport_actor.clone(),
            session_actor.clone(),
            proposal_actor.clone(),
        ));

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
