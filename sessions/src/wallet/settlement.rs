use {
    crate::{
        rpc::{RpcResponsePayload, SessionProposeRequest},
        wallet::SessionProposePublicKey,
        Result,
        WalletSettlementHandler,
    },
    monedero_domain::namespaces::Namespaces,
    std::sync::Arc,
    tokio::sync::Mutex,
    xtra::prelude::*,
};

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
    handler: Arc<Mutex<Box<dyn WalletSettlementHandler>>>,
}

impl WalletSettlementActor {
    pub fn new<T: WalletSettlementHandler>(handler: T) -> Self {
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
