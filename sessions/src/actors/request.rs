use crate::actors::session::SessionRequestHandlerActor;
use crate::actors::{RegisterDapp, RegisterWallet, SessionSettled, TransportActor};
use crate::domain::Topic;
use crate::rpc::{
    ErrorParams, PairDeleteRequest, PairPingRequest, RequestParams, ResponseParamsError,
    ResponseParamsSuccess, RpcRequest, RpcResponse, RpcResponsePayload, SessionProposeRequest,
};
use crate::{Dapp, MessageId, Result, Wallet};
use crate::{PairingManager, RegisteredManagers};
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};
use walletconnect_relay::Client;
use xtra::prelude::*;

#[derive(Clone, xtra::Actor)]
pub(crate) struct RequestHandlerActor {
    pair_managers: Arc<DashMap<Topic, Address<PairingManager>>>,
    dapps: Arc<DashMap<Topic, Address<Dapp>>>,
    wallets: Arc<DashMap<Topic, Address<Wallet>>>,
    responder: Address<TransportActor>,
    session_handler: Address<SessionRequestHandlerActor>,
}

impl Handler<RegisterWallet> for RequestHandlerActor {
    type Return = ();

    async fn handle(&mut self, message: RegisterWallet, ctx: &mut Context<Self>) -> Self::Return {
        info!("registering wallet for requests on topic {}", message.0);
        let addr = xtra::spawn_tokio(message.1, Mailbox::unbounded());
        self.wallets.insert(message.0, addr);
    }
}

impl Handler<RegisterDapp> for RequestHandlerActor {
    type Return = ();

    async fn handle(&mut self, message: RegisterDapp, ctx: &mut Context<Self>) -> Self::Return {
        let addr = xtra::spawn_tokio(message.1, Mailbox::unbounded());
        self.dapps.insert(message.0, addr);
    }
}

impl Handler<RegisteredManagers> for RequestHandlerActor {
    type Return = usize;

    async fn handle(
        &mut self,
        _message: RegisteredManagers,
        ctx: &mut Context<Self>,
    ) -> Self::Return {
        self.pair_managers.len()
    }
}

pub(crate) struct RegisterTopicManager(pub(crate) Topic, pub(crate) PairingManager);

impl Handler<RegisterTopicManager> for RequestHandlerActor {
    type Return = ();

    async fn handle(
        &mut self,
        message: RegisterTopicManager,
        ctx: &mut Context<Self>,
    ) -> Self::Return {
        tracing::info!("registering mgr for topic {}", message.0);
        let addr = xtra::spawn_tokio(message.1, Mailbox::unbounded());
        self.pair_managers.insert(message.0, addr);
    }
}

impl Handler<Client> for RequestHandlerActor {
    type Return = Result<()>;

    async fn handle(&mut self, message: Client, ctx: &mut Context<Self>) -> Self::Return {
        self.send_client(message).await
    }
}

impl RequestHandlerActor {
    pub(crate) fn new(
        responder: Address<TransportActor>,
        session_handler: Address<SessionRequestHandlerActor>,
    ) -> Self {
        Self {
            pair_managers: Arc::new(DashMap::new()),
            responder,
            session_handler,
            dapps: Arc::new(DashMap::new()),
            wallets: Arc::new(DashMap::new()),
        }
    }

    pub(crate) async fn send_client(&self, relay: Client) -> Result<()> {
        Ok(self.responder.send(relay).await?)
    }
}

async fn handle_pair_delete_request(
    args: PairDeleteRequest,
    managers: Arc<DashMap<Topic, Address<PairingManager>>>,
    responder: Address<TransportActor>,
    unknown: RpcResponse,
) {
    let id = unknown.id;
    let topic = unknown.topic.clone();
    let response: RpcResponse = match managers.get(&topic) {
        Some(mgr) => mgr
            .send(args)
            .await
            .map(|r| RpcResponse {
                id,
                topic: topic.clone(),
                payload: r,
            })
            .unwrap_or_else(|e| {
                warn!("unknown error for request {e} id:{} topic:{}", id, topic);
                unknown
            }),
        None => {
            warn!("topic {topic} has no pairing manager!");
            unknown
        }
    };
    if let Err(err) = responder.send(response).await {
        warn!(
            "Failed to send response for id {} on topic {} {}",
            id, topic, err
        );
    }
}

async fn handle_pair_request(
    args: PairPingRequest,
    managers: Arc<DashMap<Topic, Address<PairingManager>>>,
    responder: Address<TransportActor>,
    unknown: RpcResponse,
) {
    let id = unknown.id;
    let topic = unknown.topic.clone();
    let response: RpcResponse = match managers.get(&topic) {
        Some(mgr) => mgr
            .send(args)
            .await
            .map(|r| RpcResponse {
                id,
                topic: topic.clone(),
                payload: r,
            })
            .unwrap_or_else(|e| {
                warn!("unknown error for request {e} id:{} topic:{}", id, topic);
                unknown
            }),
        None => {
            warn!("topic {topic} has no pairing manager!");
            unknown
        }
    };
    if let Err(err) = responder.send(response).await {
        warn!(
            "Failed to send response for id {} on topic {} {}",
            id, topic, err
        );
    }
}

async fn process_proposal(
    actor: RequestHandlerActor,
    id: MessageId,
    topic: Topic,
    req: SessionProposeRequest,
) -> crate::Result<RpcResponse> {
    let w = actor
        .wallets
        .get(&topic)
        .ok_or(crate::Error::NoWalletHandler(topic.clone()))?;
    let payload = w.send(req).await?;
    Ok(RpcResponse { id, topic, payload })
}

impl Handler<RpcRequest> for RequestHandlerActor {
    type Return = ();

    async fn handle(&mut self, message: RpcRequest, ctx: &mut Context<Self>) -> Self::Return {
        let id = message.payload.id;
        let topic = message.topic.clone();
        let responder = self.responder.clone();
        let managers = self.pair_managers.clone();
        let session_handlers = self.session_handler.clone();
        debug!("handing request {id}");
        match message.payload.params {
            RequestParams::PairDelete(args) => {
                let unknown = RpcResponse::unknown(
                    id,
                    topic.clone(),
                    ResponseParamsError::PairDelete(ErrorParams::unknown()),
                );
                tokio::spawn(async move {
                    handle_pair_delete_request(args, managers, responder, unknown).await
                });
            }
            RequestParams::PairExtend(_) => {
                // TODO: complete
                if self
                    .responder
                    .send(RpcResponse {
                        id,
                        topic,
                        payload: RpcResponsePayload::Success(ResponseParamsSuccess::PairExtend(
                            true,
                        )),
                    })
                    .await
                    .is_err()
                {
                    warn!("failed to send PairExtend response");
                }
            }
            RequestParams::PairPing(args) => {
                let unknown = RpcResponse::unknown(
                    id,
                    topic.clone(),
                    ResponseParamsError::PairPing(ErrorParams::unknown()),
                );
                tokio::spawn(async move {
                    handle_pair_request(args, managers, responder, unknown).await
                });
            }

            RequestParams::SessionPropose(args) => {
                info!("got session proposal");
                let unknown = RpcResponse::unknown(
                    id,
                    topic.clone(),
                    ResponseParamsError::SessionPropose(ErrorParams::unknown()),
                );
                let response: RpcResponse =
                    match process_proposal(self.clone(), id, topic, args).await {
                        Ok(r) => r,
                        Err(e) => {
                            warn!("failed to get proposal response: {e}");
                            unknown
                        }
                    };
                if let Err(e) = self.responder.send(response).await {
                    warn!("responder actor is not responding {e}");
                }
            }
            RequestParams::SessionSettle(args) => {
                let unknown = RpcResponse::unknown(
                    id,
                    topic.clone(),
                    ResponseParamsError::SessionSettle(ErrorParams::unknown()),
                );
                let response: RpcResponse = match self.dapps.get(&topic) {
                    None => unknown,
                    Some(dapp) => dapp
                        .send(SessionSettled(topic.clone(), args))
                        .await
                        .map(|payload| RpcResponse { id, topic, payload })
                        .unwrap_or(unknown),
                };
                if let Err(e) = self.responder.send(response).await {
                    warn!("responder actor is not responding {e}");
                }
            }
            _ => {
                tokio::spawn(async move {
                    if let Err(e) = session_handlers.send(message).await {
                        warn!("failed to send to session handler actor {e}");
                    }
                });
            }
        }
    }
}
