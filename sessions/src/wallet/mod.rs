use crate::rpc::{
    ResponseParamsError, ResponseParamsSuccess, RpcResponsePayload, SessionProposeRequest,
    SessionProposeResponse, UNSUPPORTED_ACCOUNTS, UNSUPPORTED_CHAINS, USER_REJECTED,
};
use crate::{Pairing, PairingManager, Result};
use std::future::Future;
use std::str::FromStr;
use tracing::info;
use xtra::prelude::*;

#[derive(Clone, xtra::Actor)]
pub struct Wallet {
    manager: PairingManager,
}

async fn send_settlement(_wallet: Wallet) {
    info!("sending settlement")
}

impl Handler<SessionProposeRequest> for Wallet {
    type Return = RpcResponsePayload;

    async fn handle(
        &mut self,
        message: SessionProposeRequest,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        let wallet = self.clone();
        tokio::spawn(async move { send_settlement(wallet).await });
        match self.manager.pair_key() {
            None => {
                RpcResponsePayload::Error(ResponseParamsError::SessionPropose(USER_REJECTED.into()))
            }
            Some(_) => RpcResponsePayload::Success(ResponseParamsSuccess::SessionPropose(
                SessionProposeResponse {
                    relay: Default::default(),
                    responder_public_key: self.manager.pair_key().unwrap(),
                },
            )),
        }
    }
}

impl Wallet {
    pub fn new(manager: PairingManager) -> Self {
        Self { manager }
    }

    pub async fn pair(&self, uri: String) -> Result<Pairing> {
        let pairing = Pairing::from_str(&uri)?;
        self.manager.set_pairing(pairing.clone()).await?;
        Ok(pairing)
    }
}
