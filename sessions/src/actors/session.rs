use crate::actors::TransportActor;
use crate::rpc::RpcRequest;
use crate::session::ClientSession;
use crate::Topic;
use dashmap::DashMap;
use std::sync::Arc;
use xtra::prelude::*;

#[derive(Clone, xtra::Actor)]
pub(crate) struct SessionRequestHandlerActor {
    sessions: Arc<DashMap<Topic, ClientSession>>,
    responder: Address<TransportActor>,
}

impl SessionRequestHandlerActor {
    pub(crate) fn new(responder: Address<TransportActor>) -> Self {
        Self {
            sessions: Default::default(),
            responder,
        }
    }
}

impl Handler<ClientSession> for SessionRequestHandlerActor {
    type Return = ();

    async fn handle(&mut self, message: ClientSession, _ctx: &mut Context<Self>) -> Self::Return {
        self.sessions.insert(message.topic(), message);
    }
}

impl Handler<RpcRequest> for SessionRequestHandlerActor {
    type Return = ();

    async fn handle(&mut self, _message: RpcRequest, _ctx: &mut Context<Self>) -> Self::Return {}
}
