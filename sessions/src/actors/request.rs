use {
    crate::{
        actors::{
            actor_spawn,
            proposal::ProposalActor,
            session::SessionRequestHandlerActor,
            RegisteredComponents,
            TransportActor,
        },
        rpc::{Request, RequestParams, RpcRequest},
        spawn_task,
        PairingManager,
        Result,
    },
    monedero_relay::Client,
    std::fmt::{Debug, Formatter},
    tracing::{debug, warn},
    xtra::prelude::*,
};

#[derive(Clone, Actor)]
pub struct RequestHandlerActor {
    pub(super) pair_managers: Option<Address<PairingManager>>,
    pub(super) responder: Address<TransportActor>,
    session_handler: Address<SessionRequestHandlerActor>,
    proposal_handler: Address<ProposalActor>,
}
impl Debug for RequestHandlerActor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "actor-request pair_manager={}",
            self.pair_managers.is_some()
        )
    }
}

impl Handler<RegisteredComponents> for RequestHandlerActor {
    type Return = bool;

    async fn handle(
        &mut self,
        _message: RegisteredComponents,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        self.pair_managers.is_some()
    }
}

impl Handler<PairingManager> for RequestHandlerActor {
    type Return = ();

    async fn handle(&mut self, message: PairingManager, _ctx: &mut Context<Self>) -> Self::Return {
        let addr = actor_spawn(message);
        self.pair_managers = Some(addr);
    }
}

impl Handler<Client> for RequestHandlerActor {
    type Return = Result<()>;

    async fn handle(&mut self, message: Client, _ctx: &mut Context<Self>) -> Self::Return {
        self.send_client(message).await
    }
}

impl RequestHandlerActor {
    pub(crate) fn new(
        responder: Address<TransportActor>,
        session_handler: Address<SessionRequestHandlerActor>,
        proposal_handler: Address<ProposalActor>,
    ) -> Self {
        Self {
            pair_managers: None,
            responder,
            session_handler,
            proposal_handler,
        }
    }

    pub(crate) async fn send_client(&self, relay: Client) -> Result<()> {
        Ok(self.responder.send(relay).await?)
    }
}

impl Handler<RpcRequest> for RequestHandlerActor {
    type Return = ();

    #[tracing::instrument(level = "info", skip(_ctx), fields(message = %message))]
    async fn handle(&mut self, message: RpcRequest, _ctx: &mut Context<Self>) -> Self::Return {
        let id = message.payload.id;
        let topic = message.topic.clone();
        debug!("handing request {id}");
        match message.payload.params {
            RequestParams::PairDelete(args) => {
                self.handle_pair_mgr_request(id, topic.clone(), args).await;
            }
            RequestParams::PairExtend(args) => {
                self.handle_pair_mgr_request(id, topic.clone(), args).await;
            }
            RequestParams::PairPing(args) => {
                self.handle_pair_mgr_request(id, topic.clone(), args).await;
            }

            RequestParams::SessionPropose(args) => {
                let rpc = RpcRequest {
                    topic,
                    payload: Request {
                        id,
                        jsonrpc: message.payload.jsonrpc,
                        params: RequestParams::SessionPropose(args),
                    },
                };
                let proposal_handler = self.proposal_handler.clone();
                spawn_task(async move {
                    if let Err(e) = proposal_handler.send(rpc).await {
                        warn!("failed to send proposal {e}");
                    }
                });
            }
            RequestParams::SessionSettle(args) => {
                let rpc = RpcRequest {
                    topic: topic.clone(),
                    payload: Request {
                        id,
                        jsonrpc: message.payload.jsonrpc,
                        params: RequestParams::SessionSettle(args),
                    },
                };
                let proposal_handler = self.proposal_handler.clone();
                spawn_task(async move {
                    if let Err(e) = proposal_handler.send(rpc).await {
                        warn!("failed to send proposal {e}");
                    }
                });
            }
            _ => {
                let session_handlers = self.session_handler.clone();
                spawn_task(async move {
                    if let Err(e) = session_handlers.send(message).await {
                        warn!("failed to send to session handler actor {e}");
                    }
                });
            }
        }
    }
}
