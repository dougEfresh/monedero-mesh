use crate::domain::Topic;
use crate::rpc::{RequestParams, ResponseParams, ResponseParamsSuccess, SessionProposeResponse};
use crate::session::ClientSession;
use crate::transport::{RpcRecv, SessionRpcEvent, SessionTransport};
use crate::{Pairing, PairingManager};
use crate::{Result, WireEvent};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, oneshot};
use tokio::time::timeout;
use tracing::{info, warn};

fn clean_up(
    mgr: PairingManager,
    tx: oneshot::Sender<Result<ClientSession>>,
    broadcast_tx: broadcast::Sender<WireEvent>,
    err: crate::Error,
) {
    let _ = mgr.ciphers.reset();
    let _ = tx.send(Err(err));
    crate::send_event(&broadcast_tx, WireEvent::SettlementFailed);
}

async fn listen_for_settlement(
    mgr: PairingManager,
    broadcast_tx: broadcast::Sender<WireEvent>,
    tx: oneshot::Sender<Result<ClientSession>>,
    session_topic: Topic,
) {
    let transport = SessionTransport {
        topic: session_topic.clone(),
        transport: mgr.transport.clone(),
    };
    let mut broadcast_rx = broadcast_tx.subscribe();
    if let Err(err) = mgr.relay.subscribe(session_topic.clone()).await {
        clean_up(mgr, tx, broadcast_tx, err.into());
        return;
    }
    loop {
        match timeout(Duration::from_secs(90), broadcast_rx.recv()).await {
            Ok(result) => {
                if result.is_err() {
                    clean_up(mgr, tx, broadcast_tx, crate::Error::SettlementRecvError);
                    return;
                }
                if let Ok(WireEvent::SessionRpc(SessionRpcEvent::Settle(settlement))) = result {
                    let client_session =
                        ClientSession::new(transport.clone(), settlement.recv.namespaces.clone());
                    let resp: ResponseParamsSuccess = match tx.send(Ok(client_session)) {
                        Ok(_) => ResponseParamsSuccess::SessionSettle(true),
                        Err(_) => ResponseParamsSuccess::SessionSettle(false),
                    };
                    let rpc: RpcRecv<ResponseParamsSuccess> =
                        RpcRecv::new(settlement.id, session_topic, resp);
                    let _ = broadcast_tx.send(WireEvent::SendResponse(rpc));
                    return;
                }
            }
            Err(_) => {
                clean_up(
                    mgr,
                    tx,
                    broadcast_tx,
                    crate::Error::SessionSettlementTimeout,
                );
                return;
            }
        }
    }
}

/*
pub(crate) async fn start_settlement<S: WalletSettlementHandler>(
    mgr: PairingManager,
    mut tx: oneshot::Sender<Result<()>>,
    responder_public_key: String,
    mut handler: S,
) {
    let mut broadcast_rx = mgr.event_subscription();
    info!("waiting session proposal");

    loop {
        match timeout(Duration::from_secs(30), broadcast_rx.recv()).await {
            Ok(result) => {
                if result.is_err() {
                    warn!("[start_settlement] broadcast channel has shutdown");
                    //clean_up(mgr, tx, broadcast_tx, crate::Error::SettlementRecvError);
                    return;
                }
                if let Ok(WireEvent::SessionRpc(SessionRpcEvent::Propose(settlement))) = result {
                    let id = settlement.id.clone();
                    let pairing_topic = settlement.topic.clone();
                    match handler.handle_propose(settlement) {
                        Err(e) => {
                            let _ = tx.send(Err(e));
                            return
                        },
                        Ok(settled) => {
                            let responder = mgr.clone();
                            let result = mgr.ciphers.create_common_topic(settled.controller.public_key);
                            let response = ResponseParamsSuccess::SessionPropose(SessionProposeResponse {
                                relay: Default::default(),
                                responder_public_key: String::from(&responder_public_key)
                            });
                            if result.is_err() {
                                tokio::spawn(async move {
                                    // send error response
                                });
                                return
                            }
                            let (session_topic, _) = result.unwrap();
                            tokio::spawn(async move {
                                responder.transport.publish_response(id, pairing_topic, response).await
                            });
                        }
                    };

                }
            },
            Err(_) => {
                clean_up(
                    mgr,
                    tx,
                    broadcast_tx,
                    crate::Error::SessionSettlementTimeout,
                );
                return;
            }
        }
    }
}
*/

pub(crate) async fn process_settlement(
    mgr: PairingManager,
    tx: oneshot::Sender<Result<ClientSession>>,
    broadcast_tx: broadcast::Sender<WireEvent>,
    pairing: Arc<Pairing>,
    payload: RequestParams,
) {
    let controller: Result<SessionProposeResponse> = mgr
        .transport
        .publish_request(pairing.topic.clone(), payload)
        .await;
    if let Err(err) = controller {
        clean_up(mgr, tx, broadcast_tx, err);
        return;
    }
    let controller = controller.unwrap();
    let result = mgr
        .ciphers
        .create_common_topic(controller.responder_public_key);

    match result {
        Ok((topic, _)) => listen_for_settlement(mgr, broadcast_tx, tx, topic).await,
        Err(err) => {
            clean_up(mgr, tx, broadcast_tx, err.into());
        }
    }
}
