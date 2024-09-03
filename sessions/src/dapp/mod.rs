mod session_settle;

use std::fmt::{Debug, Formatter};
use crate::actors::{Actors, RegisterComponent, RegisterPairing, SessionSettled};
use crate::rpc::{Metadata, RequestParams, SessionProposeRequest, SessionProposeResponse, SessionSettleRequest};
use crate::session::{ClientSession, PendingSession};
use crate::{Pairing, PairingManager, PairingTopic, ProposeFuture, Result, SessionHandlers, SessionTopic};
use std::sync::Arc;
use tracing::{error, info};
use walletconnect_namespaces::Namespaces;
use x25519_dalek::PublicKey;
use crate::Error::NoPairingTopic;
use crate::transport::SessionTransport;

#[derive(Clone, xtra::Actor)]
pub struct Dapp {
    manager: PairingManager,
    pending: Arc<PendingSession>,
    md: Metadata,
}

impl Debug for Dapp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let result: String = self.manager.topic().map_or("unknown".to_string(), |topic| topic.to_string());
        write!(f, "{} pairing:{}", self.md.name, result)
    }
}

async fn await_settlement_response(
    dapp: &Dapp,
    actors: Actors,
    params: RequestParams,
) -> Result<()> {
    let response = dapp
        .manager
        .publish_request::<SessionProposeResponse>(params)
        .await?;
    let register = RegisterPairing {
        pairing: dapp.manager.pairing().unwrap(), // I must have a pair topic, otherwise I would have never got a response
        mgr: dapp.manager.clone(),
        component: RegisterComponent::Dapp(dapp.clone(), response),
    };
    actors.register_pairing(register).await?;
    Ok(())
}

#[tracing::instrument(skip(actors), level = "debug")]
async fn begin_settlement_flow(
    dapp: Dapp,
    topic: PairingTopic,
    actors: Actors,
    params: RequestParams,
) {
    if let Err(e) = await_settlement_response(&dapp, actors, params).await {
        dapp.pending.error(&topic, e);
    }
}

fn public_key(pairing: &Pairing) -> String {
    let pk = PublicKey::from(&pairing.params.sym_key);
    data_encoding::HEXLOWER_PERMISSIVE.encode(pk.as_bytes())
}

async fn finalize_restore(dapp: Dapp, pairing: Pairing, topic: SessionTopic, settled: SessionSettleRequest) -> Result<()> {
    dapp.pending.settled(&dapp.manager,topic.clone(), settled.clone(), false).await?;
    let r = RegisterPairing {
        pairing,
        mgr: dapp.manager.clone(),
        component: RegisterComponent::DappRestore(dapp.clone(), topic),
    };
    dapp.manager.actors().register_pairing(r).await?;
    Ok(())
}

impl Dapp {
    #[must_use]
    pub fn new(manager: PairingManager, md: Metadata) -> Self {
        Self {
            manager,
            pending: Arc::new(PendingSession::new()),
            md,
        }
    }

    pub async fn pair_ping(&self) -> Result<bool> {
        self.manager.ping().await
    }

    async fn restore_session<T: SessionHandlers>(&self, topic: SessionTopic, settlement: SessionSettleRequest, handlers: T) -> Result<(Pairing, ProposeFuture<Result<ClientSession>>)> {
        info!("dapp session restore");
        let pairing = self.manager.pairing().ok_or(NoPairingTopic)?;
        let rx = self.pending.add(pairing.topic.clone(), handlers);
        let dapp = self.clone();
        let p = pairing.clone();
        tokio::spawn(async move{
            if let Err(e) = finalize_restore(dapp, p, topic, settlement).await {
                error!("failed to finalize session restore!");
            }
        });
        Ok((pairing, ProposeFuture::new(rx)))
    }

    #[tracing::instrument(level = "debug" , skip(handlers, chains))]
    pub async fn propose<T: SessionHandlers>(
        &self,
        handlers: T,
        chains: impl Into<Namespaces>,
    ) -> Result<(Pairing, ProposeFuture<Result<ClientSession>>)> {
        let namespaces: Namespaces = chains.into();

        if let Some(settled) = self.manager.find_session(&namespaces) {
            return self.restore_session(settled.0, settled.1, handlers).await
        }

        let pairing = self.manager.pairing().unwrap_or(Pairing::default());
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
