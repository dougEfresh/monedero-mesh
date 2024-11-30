use {
    crate::{
        rpc::{ResponseParamsSuccess, RpcResponsePayload},
        session::Category,
        spawn_task,
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

    async fn handle(&mut self, message: SessionSettled, _ctx: &mut Context<Self>) -> Self::Return {
        let me = self.clone();
        spawn_task(async move {
            if let Err(e) = me.process_settlement(message).await {
                tracing::warn!("failed to complete settlement: {e}");
            }
        });
        RpcResponsePayload::Success(ResponseParamsSuccess::SessionSettle(true))
    }
}
