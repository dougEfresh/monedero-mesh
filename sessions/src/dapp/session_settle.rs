use crate::rpc::{ResponseParamsError, ResponseParamsSuccess, RpcResponsePayload};
use crate::transport::SessionTransport;
use crate::{Actors, ClientSession, Dapp, Result, SessionEventRequest, SessionSettled};
use tokio::sync::mpsc;
use xtra::{Context, Handler};
use crate::session::Category;

impl Dapp {
    async fn process_settlement(&self, settled: SessionSettled) -> Result<()> {
        self.pending.settled(&self.manager, settled, Category::Dapp,  None).await?;
        Ok(())
    }
}

impl Handler<SessionSettled> for Dapp {
    type Return = RpcResponsePayload;

    async fn handle(&mut self, message: SessionSettled, _ctx: &mut Context<Self>) -> Self::Return {
        match self.process_settlement(message).await {
            Ok(_) => RpcResponsePayload::Success(ResponseParamsSuccess::SessionSettle(true)),
            Err(e) => {
                tracing::warn!("failed to complete settlement: {e}");
                RpcResponsePayload::Error(ResponseParamsError::SessionSettle(
                    crate::SdkErrors::UserRejected.into(),
                ))
            }
        }
    }
}
