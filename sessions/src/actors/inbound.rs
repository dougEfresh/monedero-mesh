use {
    crate::{
        actors::{AddRequest, ClearPairing},
        rpc::Response,
    },
    dashmap::DashMap,
    monedero_domain::MessageId,
    monedero_relay::MessageIdGenerator,
    std::sync::Arc,
    tokio::sync::oneshot,
    tracing::{debug, error, warn},
    xtra::{Context, Handler},
};

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
