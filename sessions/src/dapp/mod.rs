mod session_settle;

use crate::actors::{Actors, RegisterComponent, RegisterPairing};
use crate::domain::Topic;
use crate::rpc::{Metadata, RequestParams, SessionProposeRequest, SessionProposeResponse};
use crate::session::{ClientSession, PendingSession};
use crate::transport::{SessionTransport, TopicTransport};
use crate::Error::NoPairingTopic;
use crate::{
    NoopSessionDeleteHandler, Pairing, PairingManager, PairingTopic, ProposeFuture, Result,
    SessionHandlers,
};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;
use tracing::{debug, info, warn};
use walletconnect_namespaces::Namespaces;
use x25519_dalek::PublicKey;

#[derive(Clone, xtra::Actor)]
pub struct Dapp {
    manager: PairingManager,
    pending: Arc<PendingSession>,
    md: Metadata,
}

async fn begin_settlement_flow(
    dapp: Dapp,
    topic: PairingTopic,
    actors: Actors,
    params: RequestParams,
) {
    match dapp
        .manager
        .publish_request::<SessionProposeResponse>(params)
        .await
    {
        Ok(response) => {
            info!("registering wallet's public key");
            let register = RegisterPairing {
                pairing: dapp.manager.pairing().unwrap(), // I must have a pair topic, otherwise I would never got a response
                mgr: dapp.manager.clone(),
                component: RegisterComponent::Dapp(dapp.clone(), response),
            };
            if let Err(e) = actors.register_pairing(register).await {
                dapp.pending.error(&topic, e);
            }
        }
        Err(e) => dapp.pending.error(&topic, e),
    }
}

fn public_key(pairing: &Pairing) -> String {
    let pk = PublicKey::from(&pairing.params.sym_key);
    data_encoding::HEXLOWER_PERMISSIVE.encode(pk.as_bytes())
}

/*
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
 */

impl Dapp {
    #[must_use]
    pub fn new(manager: PairingManager, md: Metadata) -> Self {
        Self {
            manager,
            pending: Arc::new(PendingSession::new()),
            md,
        }
    }

    pub async fn propose<T: SessionHandlers>(
        &self,
        handlers: T,
        chains: impl Into<Namespaces>,
    ) -> Result<(Pairing, ProposeFuture<Result<ClientSession>>)> {
        let namespaces: Namespaces = chains.into();
        let pairing = Pairing::default();
        let register = RegisterPairing {
            pairing: pairing.clone(),
            mgr: self.manager.clone(),
            component: RegisterComponent::None,
        };
        self.manager.actors().register_pairing(register).await?;
        let rx = self.pending.add(pairing.topic.clone(), handlers);
        let pk = public_key(&pairing);
        let params = RequestParams::SessionPropose(SessionProposeRequest::new(
            self.md.clone(),
            pk,
            namespaces,
            None,
        ));
        let actors = self.manager.actors();
        let dapp = self.clone();
        let topic = pairing.topic.clone();
        tokio::spawn(async move { begin_settlement_flow(dapp, topic, actors, params).await });
        Ok((pairing, ProposeFuture::new(rx)))
    }
}
