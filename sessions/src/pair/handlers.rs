use crate::rpc::{PairDeleteRequest, PairPingRequest, ResponseParamsSuccess, RpcResponsePayload};
use crate::{PairingManager, SocketEvent, Topic};
use backoff::future::retry;
use backoff::ExponentialBackoffBuilder;
use std::time::Duration;
use tracing::{info, warn};
use xtra::prelude::*;

async fn handle_socket_close(mgr: PairingManager) {
    info!("reconnecting");
    tokio::time::sleep(Duration::from_secs(3)).await;

    let backoff = ExponentialBackoffBuilder::new()
        .with_max_elapsed_time(Some(Duration::from_secs(60)))
        .with_initial_interval(Duration::from_secs(3))
        .build();
    let _ = retry(backoff, || async {
        info!("attempting reconnect");
        Ok(mgr.open_socket().await?)
    })
    .await;
}

impl Handler<SocketEvent> for PairingManager {
    type Return = ();

    async fn handle(&mut self, message: SocketEvent, ctx: &mut Context<Self>) -> Self::Return {
        info!("handling socket event {message}");
        if message == SocketEvent::ForceDisconnect {
            let mgr = self.clone();
            //TODO check if already reconnecting
            tokio::spawn(async move { handle_socket_close(mgr).await });
        }
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
        //TODO move to actor
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
