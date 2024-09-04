use crate::rpc::{
    PairDeleteRequest, PairExtendRequest, PairPingRequest, ResponseParamsSuccess,
    RpcResponsePayload,
};
use crate::{PairingManager, SocketEvent, Topic};
use backoff::future::retry;
use backoff::ExponentialBackoffBuilder;
use std::time::Duration;
use tracing::{info, warn};
use xtra::prelude::*;

impl Handler<PairExtendRequest> for PairingManager {
    type Return = RpcResponsePayload;

    async fn handle(
        &mut self,
        _message: PairExtendRequest,
        ctx: &mut Context<Self>,
    ) -> Self::Return {
        //TODO complete
        RpcResponsePayload::Success(ResponseParamsSuccess::PairExtend(true))
    }
}

impl Handler<PairPingRequest> for PairingManager {
    type Return = RpcResponsePayload;

    async fn handle(&mut self, _message: PairPingRequest, ctx: &mut Context<Self>) -> Self::Return {
        RpcResponsePayload::Success(ResponseParamsSuccess::PairPing(true))
    }
}

impl Handler<PairDeleteRequest> for PairingManager {
    type Return = RpcResponsePayload;

    async fn handle(
        &mut self,
        _message: PairDeleteRequest,
        ctx: &mut Context<Self>,
    ) -> Self::Return {
        if let Some(pairing) = self.ciphers.pairing() {
            let mgr = self.clone();
            tokio::spawn(async move {
                // Give time some time to respond to delete request
                tokio::time::sleep(Duration::from_secs(1)).await;
                //TODO unsubscribe to session topics?
                if let Err(e) = mgr.cleanup(pairing.topic).await {
                    warn!("failed to remove pairing topic from ciphers/storage {e}");
                }
            });
        }
        RpcResponsePayload::Success(ResponseParamsSuccess::PairPing(true))
    }
}

impl PairingManager {
    pub(super) async fn cleanup(&self, pairing_topic: Topic) -> crate::Result<()> {
        info!("deleting pairing topic {pairing_topic}");
        let _ = self.transport.unsubscribe(pairing_topic).await;
        self.ciphers.set_pairing(None)?;
        Ok(())
    }
}
