use crate::actors::ClearPairing;
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
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        //TODO complete
        RpcResponsePayload::Success(ResponseParamsSuccess::PairExtend(true))
    }
}

impl Handler<PairPingRequest> for PairingManager {
    type Return = RpcResponsePayload;

    async fn handle(
        &mut self,
        _message: PairPingRequest,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        RpcResponsePayload::Success(ResponseParamsSuccess::PairPing(true))
    }
}

impl Handler<PairDeleteRequest> for PairingManager {
    type Return = RpcResponsePayload;

    async fn handle(
        &mut self,
        _message: PairDeleteRequest,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        if let Some(pairing) = self.ciphers.pairing() {
            let mgr = self.clone();
            tokio::spawn(async move {
                // Give time some time to respond to delete request
                tokio::time::sleep(Duration::from_secs(1)).await;
                mgr.cleanup(pairing.topic).await;
            });
        }
        RpcResponsePayload::Success(ResponseParamsSuccess::PairPing(true))
    }
}

impl PairingManager {
    pub(super) async fn cleanup(&self, pairing_topic: Topic) {
        info!("deleting pairing topic {pairing_topic}");
        let _ = self.transport.unsubscribe(pairing_topic).await;
        let _ = self.actors.request().send(ClearPairing).await;
        let topics = self.ciphers.subscriptions();
        for t in topics {
            let _ = self.relay.unsubscribe(t).await;
        }
        let _ = self.ciphers.set_pairing(None);
    }
}
