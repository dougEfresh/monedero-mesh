mod session_settle;

use crate::rpc::{
    Metadata, RequestParams, SessionProposeRequest, SessionProposeResponse, SessionSettleRequest,
};
use crate::session::{ClientSession, PendingSession};
use crate::Error::NoPairingTopic;
use crate::{
    Pairing, PairingManager, PairingTopic, ProposeFuture, Result, SessionHandler, SessionSettled,
    SessionTopic, SocketListener,
};
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use tracing::{error, info};
use walletconnect_namespaces::Namespaces;
use x25519_dalek::PublicKey;

#[derive(Clone, xtra::Actor)]
pub struct Dapp {
    manager: PairingManager,
    pending: Arc<PendingSession>,
    md: Metadata,
}

fn common_display(dapp: &Dapp) -> String {
    format!(
        "{} pairing:{}",
        dapp.md.name,
        dapp.manager
            .topic()
            .map_or("unknown".to_string(), |topic| crate::shorten_topic(&topic))
    )
}

impl Display for Dapp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", common_display(self))
    }
}

impl Debug for Dapp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", common_display(self))
    }
}

async fn await_settlement_response(dapp: &Dapp, params: RequestParams) -> Result<()> {
    let response = dapp
        .manager
        .publish_request::<SessionProposeResponse>(params)
        .await?;
    dapp.manager.register_wallet_pk(response).await?;
    Ok(())
}

#[tracing::instrument(skip(topic, params), level = "debug")]
async fn begin_settlement_flow(dapp: Dapp, topic: PairingTopic, params: RequestParams) {
    if let Err(e) = await_settlement_response(&dapp, params).await {
        dapp.pending.error(&topic, e);
    }
}

fn public_key(pairing: &Pairing) -> String {
    let pk = PublicKey::from(&pairing.params.sym_key);
    data_encoding::HEXLOWER_PERMISSIVE.encode(pk.as_bytes())
}

async fn finalize_restore(dapp: Dapp, settled: SessionSettled) -> Result<()> {
    dapp.pending.settled(&dapp.manager, settled, None).await?;
    Ok(())
}

impl Dapp {
    pub async fn new(manager: PairingManager, md: Metadata) -> Result<Self> {
        let me = Self {
            manager,
            pending: Arc::new(PendingSession::new()),
            md,
        };
        me.manager.actors().proposal().send(me.clone()).await?;
        Ok(me)
    }

    pub async fn pair_ping(&self) -> Result<bool> {
        self.manager.ping().await
    }

    fn restore_session<T: SessionHandler>(
        &self,
        settlement: SessionSettled,
        handlers: T,
    ) -> Result<(Pairing, ProposeFuture)> {
        info!("dapp session restore");
        let pairing = self.manager.pairing().ok_or(NoPairingTopic)?;
        let rx = self.pending.add(pairing.topic.clone(), handlers);
        let dapp = self.clone();
        tokio::spawn(async move {
            if let Err(e) = finalize_restore(dapp, settlement).await {
                error!("failed to finalize session restore! {e}");
            }
        });
        Ok((pairing, ProposeFuture::new(rx)))
    }

    /// Propose
    ///
    /// Reference spec: [https://specs.walletconnect.com/2.0/specs/clients/core/pairing]
    /// This function will restore sessions if there is a matching namespace session
    /// Otherwise new pairing session will be established
    #[tracing::instrument(level = "debug", skip(handlers, chains))]
    pub async fn propose<T>(
        &self,
        handlers: T,
        chains: impl Into<Namespaces>,
    ) -> Result<(Pairing, ProposeFuture)>
    where
        T: SessionHandler,
    {
        let namespaces: Namespaces = chains.into();

        if let Some(settled) = self.manager.find_session(&namespaces) {
            return self.restore_session(settled, handlers);
        }

        let pairing = self.manager.pairing().unwrap_or_default();
        self.manager.set_pairing(pairing.clone()).await?;
        let rx = self.pending.add(pairing.topic.clone(), handlers);
        let pk = public_key(&pairing);
        let params = RequestParams::SessionPropose(SessionProposeRequest::new(
            self.md.clone(),
            pk,
            namespaces,
            None,
        ));
        let dapp = self.clone();
        let topic = pairing.topic.clone();
        tokio::spawn(async move { begin_settlement_flow(dapp, topic, params).await });
        Ok((pairing, ProposeFuture::new(rx)))
    }

    pub fn pairing(&self) -> Option<Pairing> {
        self.manager.pairing()
    }
}
