use {
    crate::{
        rpc::{ResponseParamsError, ResponseParamsSuccess, RpcResponsePayload},
        session::Category,
        transport::SessionTransport,
        Actors,
        ClientSession,
        Dapp,
        Result,
        SessionEventRequest,
        SessionSettled,
    },
    tokio::sync::mpsc,
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
}
