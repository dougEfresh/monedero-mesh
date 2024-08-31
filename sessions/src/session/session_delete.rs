use crate::rpc::{ResponseParamsSuccess, RpcResponsePayload, SessionDeleteRequest};
use crate::{ClientSession, SessionDeleteHandler};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::warn;
use xtra::prelude::*;

pub(crate) async fn handle_delete<T: SessionDeleteHandler>(
    handler: T,
    mut rx: mpsc::Receiver<SessionDeleteRequest>,
) {
    while let Some(message) = rx.recv().await {
        handler.handle(message).await;
    }
}

impl Handler<SessionDeleteRequest> for ClientSession {
    type Return = RpcResponsePayload;

    async fn handle(
        &mut self,
        message: SessionDeleteRequest,
        ctx: &mut Context<Self>,
    ) -> Self::Return {
        /*
        let session = self.clone();
        tokio::spawn(async move {
            if session.delete_sender.send(message).await.is_err() {
                warn!("failed to send delete request to handler");
            }
            // give some time to respond to delete request before I cleanup
            tokio::time::sleep(Duration::from_millis(100)).await;
            if let Err(e) = session.cleanup_session().await {
                warn!("failed to cleanup own session {} {e}", session.topic());
            }
        });
         */
        RpcResponsePayload::Success(ResponseParamsSuccess::SessionDelete(true))
    }
}
