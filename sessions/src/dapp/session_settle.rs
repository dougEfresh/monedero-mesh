use {
    crate::{
        rpc::{ResponseParamsSuccess, RpcResponsePayload},
        session::Category,
        Dapp,
        Result,
    },
    monedero_domain::SessionSettled,
    xtra::{Context, Handler},
};

impl Dapp {
    async fn process_settlement(&self, settled: SessionSettled) -> Result<()> {
        self.pending
            .settled(&self.manager, settled, Category::Dapp, None)
            .await?;
        Ok(())
    }
}

impl Handler<SessionSettled> for Dapp {
    type Return = RpcResponsePayload;

    #[cfg(not(target_family = "wasm"))]
    async fn handle(&mut self, message: SessionSettled, _ctx: &mut Context<Self>) -> Self::Return {
        use crate::rpc::ResponseParamsError;

        match self.process_settlement(message).await {
            Ok(()) => RpcResponsePayload::Success(ResponseParamsSuccess::SessionSettle(true)),
            Err(e) => {
                tracing::warn!("failed to complete settlement: {e}");
                RpcResponsePayload::Error(ResponseParamsError::SessionSettle(
                    crate::SdkErrors::UserRejected.into(),
                ))
            }
        }
    }

    #[cfg(target_family = "wasm")]
    async fn handle(&mut self, message: SessionSettled, _ctx: &mut Context<Self>) -> Self::Return {
        let me = self.clone();
        crate::spawn_task(async move {
            if let Err(e) = me.process_settlement(message).await {
                tracing::warn!("failed to complete settlement: {e}");
            }
        });
        RpcResponsePayload::Success(ResponseParamsSuccess::SessionSettle(true))
    }
}
