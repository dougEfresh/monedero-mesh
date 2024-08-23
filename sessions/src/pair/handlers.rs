use crate::rpc::{PairDeleteRequest, PairPingRequest, ResponseParamsSuccess, RpcResponsePayload};
use crate::{PairingManager, SocketEvent, SocketHandler, WireEvent};
use std::future::Future;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{info, warn};
use xtra::prelude::*;

async fn handle_socket_close(mgr: PairingManager) {
    info!("reconnecting after 3 seconds");
    tokio::time::sleep(Duration::from_secs(3)).await;
    if let Err(e) = mgr.socket_open().await {
        // backoff

        tracing::error!("failed to reconnect {e}");
    }
}

impl Handler<SocketEvent> for PairingManager {
    type Return = ();

    async fn handle(&mut self, message: SocketEvent, ctx: &mut Context<Self>) -> Self::Return {
        match message {
            SocketEvent::ForceDisconnect => {
                let mgr = self.clone();
                //TODO check if already reconnecting
                tokio::spawn(async move { handle_socket_close(mgr).await });
            }
            _ => {}
        }
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
            let relay = self.relay.clone();
            let ciphers = self.ciphers.clone();
            info!("deleting pairing topic {}", pairing.topic);
            tokio::spawn(async move {
                // Give time some time to respond to delete request
                tokio::time::sleep(Duration::from_secs(1)).await;
                //TODO unsubscribe to session topics?
                if let Err(e) = ciphers.set_pairing(None) {
                    warn!("failed to remove pairing topic from ciphers/storage {e}");
                }

                relay.unsubscribe(pairing.topic).await
            });
        }
        RpcResponsePayload::Success(ResponseParamsSuccess::PairPing(true))
    }
}
