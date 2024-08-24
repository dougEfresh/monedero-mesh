use crate::actors::Actors;
use crate::domain::Topic;
use crate::rpc::{
    ProposeNamespaces, RelayProtocol, RequestParams, ResponseParamsSuccess, RpcResponsePayload,
    SessionProposeRequest, SessionProposeResponse, SessionSettleRequest,
};
use crate::session::ClientSession;
use crate::transport::TopicTransport;
use crate::Result;
use crate::{Pairing, PairingManager};
use dashmap::DashMap;
use std::future::Future;
use std::sync::{Arc, RwLock};
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;
use tracing::{debug, error, warn};
use x25519_dalek::PublicKey;
use xtra::{Context, Handler};

#[derive(Clone, xtra::Actor)]
pub struct Dapp {
    manager: PairingManager,
    pending_proposals: Arc<DashMap<Topic, Sender<Result<ClientSession>>>>,
}

impl Handler<SessionSettleRequest> for Dapp {
    type Return = RpcResponsePayload;

    async fn handle(
        &mut self,
        message: SessionSettleRequest,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        match self.manager.topic() {
            None => {
                warn!("pairing topic is unknown, cannot complete settlement");
                RpcResponsePayload::Success(ResponseParamsSuccess::SessionSettle(false))
            }
            Some(pairing_topic) => match self.pending_proposals.remove(&pairing_topic) {
                None => {
                    warn!(
                        "no one to send client session pairing_topic={}",
                        pairing_topic
                    );
                    RpcResponsePayload::Success(ResponseParamsSuccess::SessionSettle(false))
                }
                Some((_, tx)) => {
                    let _ = tx.send(Err(crate::Error::NoPairingTopic));
                    RpcResponsePayload::Success(ResponseParamsSuccess::SessionSettle(false))
                }
            },
        }
    }
}

fn handle_error(dapp: Dapp, e: crate::Error) {
    debug!("session settlement failed {}", e);
    if let Some(topic) = dapp.manager.topic() {
        if let Some((_, mut tx)) = dapp.pending_proposals.remove(&topic) {
            if let Err(_) = tx.send(Err(e)) {
                warn!("proposal channel is gone!");
                return;
            }
        } else {
            warn!("could not find receiver for settlement")
        }
    } else {
        warn!("no pairing topic available!");
    }
}

async fn begin_settlement_flow(dapp: Dapp, actors: Actors, params: RequestParams) {
    match dapp
        .manager
        .publish_request::<SessionProposeResponse>(params)
        .await
    {
        Ok(response) => {
            if let Err(e) = actors.register_proposal_pk(dapp.clone(), response).await {
                handle_error(dapp, e)
            }
        }
        Err(e) => handle_error(dapp, e),
    }
}

fn public_key(pairing: &Pairing) -> String {
    let pk = PublicKey::from(&pairing.params.sym_key);
    data_encoding::HEXLOWER_PERMISSIVE.encode(pk.as_bytes())
}

impl Dapp {
    pub async fn propose(
        &self,
        namespaces: ProposeNamespaces,
    ) -> Result<(Pairing, oneshot::Receiver<Result<ClientSession>>)> {
        let pairing = Pairing::default();
        self.manager.set_pairing(pairing.clone()).await?;

        let (tx, rx) = oneshot::channel::<Result<ClientSession>>();
        let actors = self.manager.actors();
        let pk = public_key(&pairing);
        let params = RequestParams::SessionPropose(SessionProposeRequest::new(pk, namespaces));
        self.pending_proposals.insert(pairing.topic.clone(), tx);
        let dapp = self.clone();
        tokio::spawn(async move { begin_settlement_flow(dapp, actors, params).await });
        Ok((pairing, rx))
    }
}
