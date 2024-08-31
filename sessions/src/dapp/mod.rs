mod session_settle;

use crate::actors::Actors;
use crate::domain::Topic;
use crate::rpc::{
    Metadata, ProposeNamespaces, RequestParams, SessionProposeRequest, SessionProposeResponse,
    SettleNamespaces,
};
use crate::session::ClientSession;
use crate::transport::{SessionTransport, TopicTransport};
use crate::{NoopSessionDeleteHandler, Pairing, PairingManager, ProposeFuture, Result};
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
    md: Metadata,
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
        warn!("could not find receiver for settlement");
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

fn restore_session(
    tx: Sender<Result<ClientSession>>,
    actors: Actors,
    transport: SessionTransport,
    namespaces: SettleNamespaces,
) {
    let session = ClientSession::new(
        actors.cipher_actor(),
        transport,
        namespaces,
        NoopSessionDeleteHandler,
    );
    if tx.send(Ok(session)).is_err() {
        warn!("settlement oneshoot has closed");
    }
}

impl Dapp {
    #[must_use]
    pub fn new(manager: PairingManager, md: Metadata) -> Self {
        Self {
            manager,
            pending_proposals: Arc::new(DashMap::new()),
            md,
        }
    }

    pub fn propose(
        &self,
        namespaces: ProposeNamespaces,
    ) -> Result<(Pairing, ProposeFuture<Result<ClientSession>>)> {
        let pairing = Pairing::default();
        let (tx, rx) = oneshot::channel::<Result<ClientSession>>();

        if let Some((transport, namespaces)) = self.manager.find_session(&namespaces)? {}

        let actors = self.manager.actors();
        let pk = public_key(&pairing);
        let params = RequestParams::SessionPropose(SessionProposeRequest::new(
            self.md.clone(),
            pk,
            namespaces,
        ));
        self.pending_proposals.insert(pairing.topic.clone(), tx);
        let dapp = self.clone();
        tokio::spawn(async move { begin_settlement_flow(dapp, actors, params).await });
        Ok((pairing, ProposeFuture::new(rx)))
    }
}
