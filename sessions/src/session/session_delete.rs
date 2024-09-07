use crate::rpc::{ResponseParamsSuccess, RpcResponsePayload, SessionDeleteRequest};
use crate::{ClientSession, SessionDeleteHandler};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn};
use xtra::prelude::*;

#[allow(dead_code)]
pub async fn handle_delete<T: SessionDeleteHandler>(
    handler: T,
    mut rx: mpsc::Receiver<SessionDeleteRequest>,
) {
    while let Some(message) = rx.recv().await {
        handler.handle(message).await;
    }
}

impl Handler<SessionDeleteRequest> for ClientSession {
    type Return = ();

    async fn handle(
        &mut self,
        message: SessionDeleteRequest,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        info!("session delete requested {message:#?}");
    }
}
