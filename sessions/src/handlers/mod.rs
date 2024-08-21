use crate::rpc::RequestParams;
use crate::transport::{PairingRpcEvent, RpcRecv, SessionRpcEvent};
use crate::WireEvent;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::{RecvError, SendError};
use tracing::warn;

pub(crate) async fn rpc_recv_event(tx: broadcast::Sender<WireEvent>) {
    let mut rx = tx.subscribe();

    loop {
        match rx.recv().await {
            Err(_) => {
                if let Err(err) = rx.recv().await {
                    if err == RecvError::Closed {
                        return;
                    }
                    warn!("lagging broadcast channel!");
                }
            }
            Ok(event) => {
                if let WireEvent::RequestRecv(rpc) = event {
                    match &rpc.recv {
                        RequestParams::PairDelete(_)
                        | RequestParams::PairExtend(_)
                        | RequestParams::PairPing(_) => {
                            let pair_event: PairingRpcEvent = rpc.into();
                            crate::send_event(&tx, WireEvent::PairingRpc(pair_event));
                        }
                        RequestParams::SessionPropose(_)
                        | RequestParams::SessionSettle(_)
                        | RequestParams::SessionExtend(_)
                        | RequestParams::SessionRequest(_)
                        | RequestParams::SessionEvent(_)
                        | RequestParams::SessionDelete(_)
                        | RequestParams::SessionPing(_)
                        | RequestParams::SessionUpdate(_) => {
                            let session_event: SessionRpcEvent = rpc.into();
                            crate::send_event(&tx, WireEvent::SessionRpc(session_event));
                        }
                    }
                }
            }
        }
    }
}
