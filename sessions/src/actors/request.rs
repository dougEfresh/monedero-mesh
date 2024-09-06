use crate::actors::proposal::ProposalActor;
use crate::actors::session::SessionRequestHandlerActor;
use crate::actors::{
    ClearPairing, RegisterDapp, RegisterTopicManager, RegisterWallet, RegisteredComponents,
    TransportActor,
};
use crate::domain::Topic;
use crate::rpc::{
    ErrorParams, IntoUnknownError, PairPingRequest, Request, RequestParams, ResponseParamsError,
    ResponseParamsSuccess, RpcRequest, RpcResponse, RpcResponsePayload, SessionProposeRequest,
};
use crate::PairingManager;
use crate::{Dapp, MessageId, Result, Wallet};
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};
use walletconnect_relay::Client;
use xtra::prelude::*;

#[derive(Clone, Actor)]
pub struct RequestHandlerActor {
    pub(super) pair_managers: Arc<DashMap<Topic, Address<PairingManager>>>,
    dapps: Arc<DashMap<Topic, Address<Dapp>>>,
    wallets: Arc<DashMap<Topic, Address<Wallet>>>,
    pub(super) responder: Address<TransportActor>,
    session_handler: Address<SessionRequestHandlerActor>,
    proposal_handler: Address<ProposalActor>,
}

impl Handler<RegisteredComponents> for RequestHandlerActor {
    type Return = usize;
    async fn handle(
        &mut self,
        _message: RegisteredComponents,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        self.dapps.len() + self.wallets.len() + self.pair_managers.len()
    }
}

impl Handler<ClearPairing> for RequestHandlerActor {
    type Return = ();

    async fn handle(&mut self, _message: ClearPairing, _ctx: &mut Context<Self>) -> Self::Return {
        if let Err(e) = self.responder.send(ClearPairing).await {
            warn!("failed to cleanup transport actor: {e}");
        }
        if let Err(e) = self.session_handler.send(ClearPairing).await {
            warn!("failed to cleanup session handler: {e}");
        }
        self.pair_managers.clear();
        self.wallets.clear();
        self.dapps.clear();
    }
}

impl Handler<RegisterWallet> for RequestHandlerActor {
    type Return = ();

    async fn handle(&mut self, message: RegisterWallet, _ctx: &mut Context<Self>) -> Self::Return {
        info!("registering wallet for requests on topic {}", message.0);
        let addr = xtra::spawn_tokio(message.1, Mailbox::unbounded());
        self.wallets.insert(message.0, addr);
    }
}

impl Handler<RegisterDapp> for RequestHandlerActor {
    type Return = ();

    async fn handle(&mut self, message: RegisterDapp, _ctx: &mut Context<Self>) -> Self::Return {
        if !self.dapps.contains_key(&message.0) {
            let addr = xtra::spawn_tokio(message.1, Mailbox::unbounded());
            self.dapps.insert(message.0, addr);
        }
    }
}

impl Handler<RegisterTopicManager> for RequestHandlerActor {
    type Return = ();

    async fn handle(
        &mut self,
        message: RegisterTopicManager,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        tracing::info!("registering mgr for topic {}", message.0);
        let addr = xtra::spawn_tokio(message.1, Mailbox::unbounded());
        self.pair_managers.insert(message.0, addr);
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
            pair_managers: Arc::new(DashMap::new()),
            responder,
            session_handler,
            dapps: Arc::new(DashMap::new()),
            wallets: Arc::new(DashMap::new()),
            proposal_handler,
        }
    }

    pub(crate) async fn send_client(&self, relay: Client) -> Result<()> {
        Ok(self.responder.send(relay).await?)
    }
}

async fn process_proposal(
    actor: RequestHandlerActor,
    id: MessageId,
    topic: Topic,
    req: SessionProposeRequest,
) -> Result<RpcResponse> {
    let w = actor
        .wallets
        .get(&topic)
        .ok_or(crate::Error::NoWalletHandler(topic.clone()))?;
    let payload = w.send(req).await?;
    Ok(RpcResponse { id, topic, payload })
}

impl Handler<RpcRequest> for RequestHandlerActor {
    type Return = ();

    async fn handle(&mut self, message: RpcRequest, _ctx: &mut Context<Self>) -> Self::Return {
        let id = message.payload.id;
        let topic = message.topic.clone();
        debug!("handing request {id}");
        match message.payload.params {
            RequestParams::PairDelete(args) => {
                self.handle_pair_mgr_request(id, topic.clone(), args).await
            }
            RequestParams::PairExtend(args) => {
                self.handle_pair_mgr_request(id, topic.clone(), args).await
            }
            RequestParams::PairPing(args) => {
                self.handle_pair_mgr_request(id, topic.clone(), args).await
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
                tokio::spawn(async move {
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
                tokio::spawn(async move {
                    if let Err(e) = proposal_handler.send(rpc).await {
                        warn!("failed to send proposal {e}");
                    }
                });
            }
            _ => {
                let session_handlers = self.session_handler.clone();
                tokio::spawn(async move {
                    if let Err(e) = session_handlers.send(message).await {
                        warn!("failed to send to session handler actor {e}");
                    }
                });
            }
        }
    }
}
