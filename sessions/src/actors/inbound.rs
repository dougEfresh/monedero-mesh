use crate::actors::{AddRequest, ClearPairing};
use crate::domain::MessageId;
use crate::rpc::Response;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{debug, error, warn};
use monedero_relay::MessageIdGenerator;
use xtra::{Context, Handler};

#[derive(Default, xtra::Actor)]
pub struct InboundResponseActor {
    pending: Arc<DashMap<MessageId, oneshot::Sender<Response>>>,
    generator: MessageIdGenerator,
}

impl Handler<ClearPairing> for InboundResponseActor {
    type Return = ();

    async fn handle(&mut self, message: ClearPairing, _ctx: &mut Context<Self>) -> Self::Return {
        self.pending.clear();
    }
}

impl Handler<AddRequest> for InboundResponseActor {
    type Return = (MessageId, oneshot::Receiver<Response>);

    async fn handle(&mut self, message: AddRequest, _ctx: &mut Context<Self>) -> Self::Return {
        let id = self.generator.next();
        let (tx, rx) = oneshot::channel::<Response>();
        self.pending.insert(id, tx);
        (id, rx)
    }
}

impl Handler<Response> for InboundResponseActor {
    type Return = ();

    async fn handle(&mut self, message: Response, _ctx: &mut Context<Self>) -> Self::Return {
        debug!("handing a response with message id {}", message.id);
        if let Some((_, tx)) = self.pending.remove(&message.id) {
            let id = message.id;
            if tx.send(message).is_err() {
                warn!("oneshot channel for id {} hash closed", id);
            }
            return;
        }
        error!(
            "id [{}] not found for message {:#?}",
            message.id, message.params
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::actors::inbound::{AddRequest, InboundResponseActor};
    use crate::rpc::{
        RelayProtocolHelpers, ResponseParams, ResponseParamsSuccess, SessionProposeResponse,
    };
    use anyhow::format_err;
    use std::time::Duration;
    use xtra::prelude::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_payload_response() -> anyhow::Result<()> {
        crate::test::init_tracing();
        let addr = xtra::spawn_tokio(InboundResponseActor::default(), Mailbox::unbounded());
        let (id, rx) = addr.send(AddRequest).await?;
        let addr_resp = addr.clone();

        let params = ResponseParamsSuccess::SessionPropose(SessionProposeResponse {
            responder_public_key: "blah".to_string(),
            relay: Default::default(),
        });
        let v = serde_json::to_value(params.clone())?;
        tokio::spawn(async move {
            let resp = Response::new(id, ResponseParams::Success(v));
            addr_resp.send(resp).await
        });
        let result = tokio::time::timeout(Duration::from_millis(300), rx).await??;
        match result.params {
            ResponseParams::Success(v) => {
                let s: ResponseParamsSuccess = ResponseParamsSuccess::irn_try_from_tag(
                    v,
                    crate::rpc::TAG_SESSION_PROPOSE_RESPONSE,
                )?;
                match s {
                    ResponseParamsSuccess::SessionPropose(result) => {
                        assert_eq!("blah".to_string(), result.responder_public_key);
                        Ok(())
                    }
                    _ => Err(format_err!("expected SessionPropose")),
                }
            }
            ResponseParams::Err(_) => Err(format_err!("expected success")),
        }
    }
}
