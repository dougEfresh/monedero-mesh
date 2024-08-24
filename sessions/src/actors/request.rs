use crate::actors::{RegisterDapp, RegisterWallet, TransportActor};
use crate::domain::{MessageId, Topic};
use crate::relay::Client;
use crate::rpc::{
    ErrorParams, PairDeleteRequest, PairPingRequest, Request, RequestParams, Response,
    ResponseParams, ResponseParamsError, ResponseParamsSuccess, RpcErrorResponse, RpcRequest,
    RpcResponse, RpcResponsePayload,
};
use crate::transport::{PairingRpcEvent, RpcRecv};
use crate::{rpc, Cipher, Error, PairingManager};
use crate::{Dapp, Result, Wallet};
use dashmap::DashMap;
use serde_json::json;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tracing::{error, warn};
use xtra::prelude::*;

#[derive(xtra::Actor)]
pub(crate) struct RequestHandlerActor {
    pair_managers: Arc<DashMap<Topic, Address<PairingManager>>>,
    dapps: Arc<DashMap<Topic, Address<Dapp>>>,
    wallets: Arc<DashMap<Topic, Address<Wallet>>>,
    responder: Address<TransportActor>,
}

pub(crate) struct RegisteredManagers;

impl Handler<RegisterWallet> for RequestHandlerActor {
    type Return = ();

    async fn handle(&mut self, message: RegisterWallet, ctx: &mut Context<Self>) -> Self::Return {
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
        _ctx: &mut Context<Self>,
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
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        tracing::info!("registering mgr for topic {}", message.0);
        let addr = xtra::spawn_tokio(message.1, Mailbox::unbounded());
        self.pair_managers.insert(message.0, addr);
    }
}

impl Handler<Client> for RequestHandlerActor {
    type Return = crate::Result<()>;

    async fn handle(&mut self, message: Client, ctx: &mut Context<Self>) -> Self::Return {
        self.send_client(message).await
    }
}

impl RequestHandlerActor {
    pub(crate) fn new(responder: Address<TransportActor>) -> Self {
        Self {
            pair_managers: Arc::new(DashMap::new()),
            responder,
            dapps: Arc::new(DashMap::new()),
            wallets: Arc::new(DashMap::new()),
        }
    }

    pub(crate) async fn send_client(&self, relay: Client) -> crate::Result<()> {
        Ok(self.responder.send(relay).await?)
    }
}

async fn handle_pair_delete_request(
    args: PairDeleteRequest,
    managers: Arc<DashMap<Topic, Address<PairingManager>>>,
    responder: Address<TransportActor>,
    unknown: RpcResponse,
) {
    let id = unknown.id.clone();
    let topic = unknown.topic.clone();
    let response: RpcResponse = match managers.get(&topic) {
        Some(mgr) => mgr
            .send(args)
            .await
            .map(|r| RpcResponse {
                id: id.clone(),
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
    let id = unknown.id.clone();
    let topic = unknown.topic.clone();
    let response: RpcResponse = match managers.get(&topic) {
        Some(mgr) => mgr
            .send(args)
            .await
            .map(|r| RpcResponse {
                id: id.clone(),
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

impl Handler<RpcRequest> for RequestHandlerActor {
    type Return = ();

    async fn handle(&mut self, message: RpcRequest, _ctx: &mut Context<Self>) -> Self::Return {
        let id = message.payload.id.clone();
        let topic = message.topic.clone();
        let responder = self.responder.clone();
        let managers = self.pair_managers.clone();
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
                if let Err(_) = self
                    .responder
                    .send(RpcResponse {
                        id,
                        topic,
                        payload: RpcResponsePayload::Success(ResponseParamsSuccess::PairExtend(
                            true,
                        )),
                    })
                    .await
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
            RequestParams::SessionPropose(args) => {}
            RequestParams::SessionSettle(args) => {
                let unknown = RpcResponse::unknown(
                    id,
                    topic.clone(),
                    ResponseParamsError::SessionSettle(ErrorParams::unknown()),
                );
                let response: RpcResponse = match self.dapps.get(&topic) {
                    None => unknown,
                    Some(dapp) => dapp
                        .send(args)
                        .await
                        .map(|r| RpcResponse {
                            id,
                            topic,
                            payload: r,
                        })
                        .unwrap_or(unknown),
                };
                if let Err(e) = self.responder.send(response).await {
                    warn!("responder actor is not responding {e}");
                }
            }
            RequestParams::SessionUpdate(_) => {}
            RequestParams::SessionExtend(_) => {}
            RequestParams::SessionRequest(_) => {}
            RequestParams::SessionEvent(_) => {}
            RequestParams::SessionDelete(_) => {}
            RequestParams::SessionPing(_) => {}
        }
    }
}

#[cfg(test)]
mod test {

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_request_actor() -> anyhow::Result<()> {
        Ok(())
    }
}
