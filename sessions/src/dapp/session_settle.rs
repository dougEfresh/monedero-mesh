use crate::actors::SessionSettled;
use crate::rpc::{ResponseParamsSuccess, RpcResponsePayload};
use crate::Dapp;
use tracing::warn;
use xtra::{Context, Handler};

impl Handler<SessionSettled> for Dapp {
    type Return = RpcResponsePayload;

    async fn handle(&mut self, message: SessionSettled, _ctx: &mut Context<Self>) -> Self::Return {
        match self.manager.topic() {
            None => {
                warn!("pairing topic is unknown, cannot complete settlement");
                RpcResponsePayload::Success(ResponseParamsSuccess::SessionSettle(false))
            }
            Some(pairing_topic) => match self.pending_proposals.remove(&pairing_topic) {
                None => {
                    warn!(
                        "no one to send client session pairing_topic={}",
                        pairing_topic
                    );
                    RpcResponsePayload::Success(ResponseParamsSuccess::SessionSettle(false))
                }
                Some((_, tx)) => {
                    let session = self
                        .manager
                        .actors()
                        .register_settlement(self.manager.topic_transport(), message)
                        .await;
                    let resp = RpcResponsePayload::Success(ResponseParamsSuccess::SessionSettle(
                        session.is_ok(),
                    ));

                    tokio::spawn(async move {
                        if tx.send(session).is_err() {
                            warn!("failed to send final client session for settlement");
                        }
                    });
                    resp
                }
            },
        }
    }
}
