use crate::domain::{MessageId, Topic};
use crate::relay::MessageIdGenerator;
use crate::rpc::{RequestParams, Response, ResponseParams};
use dashmap::DashMap;
use serde_json::Value;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{error, warn};
use xtra::{Context, Handler};

#[derive(Default, xtra::Actor)]
pub(crate) struct InboundResponseActor {
    pending: Arc<DashMap<MessageId, oneshot::Sender<Response>>>,
    generator: MessageIdGenerator,
}

pub(crate) struct AddRequest;

impl Handler<AddRequest> for InboundResponseActor {
    type Return = (MessageId, oneshot::Receiver<Response>);

    async fn handle(&mut self, _message: AddRequest, _ctx: &mut Context<Self>) -> Self::Return {
        let id = self.generator.next();
        let (tx, rx) = oneshot::channel::<Response>();
        self.pending.insert(id.clone(), tx);
        (id, rx)
    }
}

impl Handler<Response> for InboundResponseActor {
    type Return = ();

    async fn handle(&mut self, message: Response, _ctx: &mut Context<Self>) -> Self::Return {
        if let Some((_, tx)) = self.pending.remove(&message.id) {
            let id = message.id.clone();
            if let Err(_) = tx.send(message) {
                warn!("oneshot channel for id {} ", id);
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
    use crate::rpc::{RelayProtocolHelpers, ResponseParamsSuccess, SessionProposeResponse};
    use anyhow::format_err;
    use std::time::Duration;
    use xtra::prelude::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_payload_response() -> anyhow::Result<()> {
        crate::tests::init_tracing();
        let addr = xtra::spawn_tokio(InboundResponseActor::default(), Mailbox::unbounded());
        let (id, rx) = addr.send(AddRequest).await?;
        let addr_resp = addr.clone();

        let params = ResponseParamsSuccess::SessionPropose(SessionProposeResponse {
            responder_public_key: "blah".to_string(),
            relay: Default::default(),
        });
        let send_purpose = params.clone();
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
