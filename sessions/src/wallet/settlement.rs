use crate::rpc::{RpcResponsePayload, SessionProposeRequest};
use crate::wallet::SessionProposePublicKey;
use crate::{Result, WalletProposalHandler};
use std::sync::Arc;
use tokio::sync::Mutex;
use walletconnect_namespaces::Namespaces;
use xtra::prelude::*;

impl Handler<SessionProposePublicKey> for WalletSettlementActor {
    type Return = (bool, RpcResponsePayload);

    async fn handle(
        &mut self,
        message: SessionProposePublicKey,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        let l = self.handler.lock().await;
        l.verify_settlement(message.1, message.0).await
    }
}

#[derive(Clone, Actor)]
pub struct WalletSettlementActor {
    handler: Arc<Mutex<Box<dyn WalletProposalHandler>>>,
}

impl WalletSettlementActor {
    pub fn new<T: WalletProposalHandler>(handler: T) -> Self {
        Self {
            handler: Arc::new(Mutex::new(Box::new(handler))),
        }
    }
}

impl Handler<SessionProposeRequest> for WalletSettlementActor {
    type Return = Result<Namespaces>;
    async fn handle(
        &mut self,
        message: SessionProposeRequest,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        let l = self.handler.lock().await;
        l.settlement(message).await
    }
}
