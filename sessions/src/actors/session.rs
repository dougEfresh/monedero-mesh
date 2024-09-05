use crate::actors::{ClearPairing, RequestHandlerActor, SessionPing, TransportActor};
use crate::rpc::{
    ErrorParams, RequestParams, ResponseParamsError, ResponseParamsSuccess, RpcRequest,
    RpcResponse, RpcResponsePayload,
};
use crate::session::ClientSession;
use crate::Topic;
use dashmap::DashMap;
use serde_json::json;
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

impl Handler<ClearPairing> for SessionRequestHandlerActor {
    type Return = ();

    async fn handle(&mut self, message: ClearPairing, _ctx: &mut Context<Self>) -> Self::Return {
        self.sessions.clear();
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
            RequestParams::SessionUpdate(args) => {
                tracing::info!("SessionEvent request {args:#?}");
                let response = RpcResponse {
                    id: message.payload.id,
                    topic: message.topic,
                    payload: RpcResponsePayload::Success(ResponseParamsSuccess::SessionUpdate(
                        true,
                    )),
                };
                if let Err(e) = self.responder.send(response).await {
                    warn!("responder actor is not responding {e}");
                }
            }
            RequestParams::SessionExtend(args) => {
                tracing::info!("SessionEvent request {args:#?}");
                let response = RpcResponse {
                    id: message.payload.id,
                    topic: message.topic,
                    payload: RpcResponsePayload::Success(ResponseParamsSuccess::SessionExtend(
                        true,
                    )),
                };
                if let Err(e) = self.responder.send(response).await {
                    warn!("responder actor is not responding {e}");
                }
            }
            RequestParams::SessionRequest(args) => {
                tracing::info!("SessionEvent request {args:#?}");
                let response = RpcResponse {
                    id: message.payload.id,
                    topic: message.topic,
                    payload: RpcResponsePayload::Success(ResponseParamsSuccess::SessionRequest(
                        json!({}),
                    )),
                };
                if let Err(e) = self.responder.send(response).await {
                    warn!("responder actor is not responding {e}");
                }
            }
            RequestParams::SessionEvent(args) => {
                tracing::info!("SessionEvent request {args:#?}");
                let response = RpcResponse {
                    id: message.payload.id,
                    topic: message.topic,
                    payload: RpcResponsePayload::Success(ResponseParamsSuccess::SessionEvent(true)),
                };
                if let Err(e) = self.responder.send(response).await {
                    warn!("responder actor is not responding {e}");
                }
            }
            RequestParams::SessionDelete(args) => {
                let unknown = RpcResponse::unknown(
                    message.payload.id,
                    message.topic.clone(),
                    ResponseParamsError::SessionDelete(ErrorParams::unknown()),
                );
                let response: RpcResponse = match self.sessions.get(&message.topic) {
                    None => unknown,
                    Some(cs) => cs
                        .send(args)
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
            RequestParams::SessionPing(_) => {
                let unknown = RpcResponse::unknown(
                    message.payload.id,
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
