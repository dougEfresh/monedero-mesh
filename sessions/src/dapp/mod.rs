mod session_settle;

use crate::actors::Actors;
use crate::domain::Topic;
use crate::rpc::{
    ProposeFuture, ProposeNamespaces, RequestParams, SessionProposeRequest, SessionProposeResponse,
};
use crate::session::ClientSession;
use crate::Result;
use crate::{Pairing, PairingManager};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;
use tracing::{debug, info, warn};
use x25519_dalek::PublicKey;

#[derive(Clone, xtra::Actor)]
pub struct Dapp {
    manager: PairingManager,
    pending_proposals: Arc<DashMap<Topic, Sender<Result<ClientSession>>>>,
}

fn handle_error(dapp: Dapp, e: crate::Error) {
    debug!("session settlement failed {}", e);
    if let Some(topic) = dapp.manager.topic() {
        if let Some((_, tx)) = dapp.pending_proposals.remove(&topic) {
            if tx.send(Err(e)).is_err() {
                warn!("proposal channel is gone!");
            }
            return;
        }
        warn!("could not find receiver for settlement")
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
            info!("registering controller's public key");
            if let Err(e) = actors.register_wallet_pk(dapp.clone(), response).await {
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
    pub fn new(manager: PairingManager) -> Self {
        Self {
            manager,
            pending_proposals: Arc::new(DashMap::new()),
        }
    }

    pub async fn propose(
        &self,
        namespaces: ProposeNamespaces,
    ) -> Result<(Pairing, ProposeFuture<Result<ClientSession>>)> {
        let pairing = Pairing::default();
        self.manager.set_pairing(pairing.clone()).await?;

        let (tx, rx) = oneshot::channel::<Result<ClientSession>>();

        let actors = self.manager.actors();
        let pk = public_key(&pairing);
        let params = RequestParams::SessionPropose(SessionProposeRequest::new(pk, namespaces));
        self.pending_proposals.insert(pairing.topic.clone(), tx);
        let dapp = self.clone();
        tokio::spawn(async move { begin_settlement_flow(dapp, actors, params).await });
        Ok((pairing, ProposeFuture::new(rx)))
    }
}
