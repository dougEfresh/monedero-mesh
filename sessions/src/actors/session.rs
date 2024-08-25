use crate::actors::{SessionPing, TransportActor};
use crate::rpc::{ErrorParams, RequestParams, ResponseParamsError, RpcRequest, RpcResponse};
use crate::session::ClientSession;
use crate::Topic;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::warn;
use xtra::prelude::*;

#[derive(Clone, xtra::Actor)]
pub(crate) struct SessionRequestHandlerActor {
    sessions: Arc<DashMap<Topic, Address<ClientSession>>>,
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
        let topic = message.topic();
        let addr = xtra::spawn_tokio(message, Mailbox::unbounded());
        self.sessions.insert(topic, addr);
    }
}

impl Handler<RpcRequest> for SessionRequestHandlerActor {
    type Return = ();

    async fn handle(&mut self, message: RpcRequest, _ctx: &mut Context<Self>) -> Self::Return {
        match message.payload.params {
            RequestParams::SessionUpdate(_) => {}
            RequestParams::SessionExtend(_) => {}
            RequestParams::SessionRequest(_) => {}
            RequestParams::SessionEvent(_) => {}
            RequestParams::SessionDelete(_) => {}
            RequestParams::SessionPing(_) => {
                let unknown = RpcResponse::unknown(
                    message.payload.id.clone(),
                    message.topic.clone(),
                    ResponseParamsError::SessionPing(ErrorParams::unknown()),
                );
                let response: RpcResponse = match self.sessions.get(&message.topic) {
                    None => unknown,
                    Some(cs) => cs
                        .send(SessionPing)
                        .await
                        .map(|payload| RpcResponse {
                            id: message.payload.id,
                            topic: message.topic,
                            payload,
                        })
                        .unwrap_or(unknown),
                };
                if let Err(e) = self.responder.send(response).await {
                    warn!("responder actor is not responding {e}");
                }
            }
            _ => warn!(
                "session request actor should not have received request {:#?}",
                message
            ),
        }
    }
}
