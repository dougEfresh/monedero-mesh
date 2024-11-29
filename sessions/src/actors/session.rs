use {
    crate::{
        actors::{actor_spawn, ClearPairing, ClearSession, SessionPing, TransportActor},
        rpc::{
            ErrorParams,
            RequestParams,
            ResponseParamsError,
            ResponseParamsSuccess,
            RpcRequest,
            RpcResponse,
            RpcResponsePayload,
        },
        session::ClientSession,
        spawn_task,
        RegisteredComponents,
        Topic,
    },
    dashmap::DashMap,
    monedero_cipher::Cipher,
    std::{
        fmt::{Debug, Formatter},
        sync::Arc,
        time::Duration,
    },
    tracing::{error, warn},
    xtra::prelude::*,
};

#[derive(Clone, xtra::Actor)]
pub struct SessionRequestHandlerActor {
    // add dapp actor here
    pub(super) sessions: Arc<DashMap<Topic, Address<ClientSession>>>,
    pub(super) responder: Address<TransportActor>,
    pub(super) cipher: Cipher,
}

impl Debug for SessionRequestHandlerActor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "actor-session sessions:{}", self.sessions.len())
    }
}

impl SessionRequestHandlerActor {
    pub(crate) fn new(responder: Address<TransportActor>, cipher: Cipher) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            responder,
            cipher,
        }
    }
}

impl Handler<ClearSession> for SessionRequestHandlerActor {
    type Return = ();

    async fn handle(&mut self, message: ClearSession, ctx: &mut Context<Self>) -> Self::Return {
        self.handle_session_delete(message.0).await;
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

    #[tracing::instrument(skip(_ctx), level = "debug")]
    async fn handle(&mut self, message: ClientSession, _ctx: &mut Context<Self>) -> Self::Return {
        let topic = message.topic();
        let addr = actor_spawn(message.clone());
        self.sessions.insert(topic.clone(), addr);
        if let Err(e) = self
            .cipher
            .set_settlement(&topic, (*message.settled).clone())
        {
            error!("failed to set settlement for {topic}");
        }
    }
}

impl Handler<RegisteredComponents> for SessionRequestHandlerActor {
    type Return = usize;

    async fn handle(
        &mut self,
        _message: RegisteredComponents,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        self.sessions.len()
    }
}

impl Handler<RpcRequest> for SessionRequestHandlerActor {
    type Return = ();

    #[tracing::instrument(skip(_ctx), level = "info", fields(message = message.to_string()))]
    async fn handle(&mut self, message: RpcRequest, _ctx: &mut Context<Self>) -> Self::Return {
        match message.payload.params {
            RequestParams::SessionUpdate(args) => {
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
                self.handle_session_request(message.payload.id, message.topic, args)
                    .await;
            }
            RequestParams::SessionEvent(args) => {
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
                if let Err(e) = self
                    .responder
                    .send(RpcResponse {
                        id: message.payload.id,
                        topic: message.topic.clone(),
                        payload: RpcResponsePayload::Success(ResponseParamsSuccess::SessionDelete(
                            true,
                        )),
                    })
                    .await
                {
                    warn!("failed to send response back for delete request {e}");
                }
                let me = self.clone();
                spawn_task(async move {
                    // give some time for the response above, before I unsubscribe.
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    me.handle_session_delete(message.topic).await;
                });
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
