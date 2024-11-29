use {
    crate::{
        actors::{InboundResponseActor, RequestHandlerActor},
        rpc::{Payload, Response, RpcRequest},
        spawn_task,
        SocketEvent,
    },
    monedero_cipher::Cipher,
    monedero_domain::Message,
    monedero_relay::{ClientError, CloseFrame, ConnectionHandler},
    tokio::sync::mpsc,
    tracing::{error, info, trace, warn},
    xtra::prelude::*,
};

pub struct RelayHandler {
    cipher: Cipher,
    req_tx: mpsc::UnboundedSender<RpcRequest>,
    res_tx: mpsc::UnboundedSender<Response>,
    socket_tx: mpsc::UnboundedSender<SocketEvent>,
}

impl RelayHandler {
    pub(crate) fn new(
        cipher: Cipher,
        request_actor: Address<RequestHandlerActor>,
        response_actor: Address<InboundResponseActor>,
        socket_tx: mpsc::UnboundedSender<SocketEvent>,
    ) -> Self {
        let (req_tx, req_rx) = mpsc::unbounded_channel::<RpcRequest>();
        let (res_tx, res_rx) = mpsc::unbounded_channel::<Response>();
        spawn_task(async move {
            event_loop_request(req_rx, request_actor).await;
        });
        spawn_task(async move {
            event_loop_response(res_rx, response_actor).await;
        });
        Self {
            cipher,
            req_tx,
            res_tx,
            socket_tx,
        }
    }
}

impl ConnectionHandler for RelayHandler {
    fn connected(&mut self) {
        if self.socket_tx.send(SocketEvent::Connected).is_err() {
            warn!("failed to send socket event");
        }
    }

    fn disconnected(&mut self, _frame: Option<CloseFrame<'static>>) {
        if self.socket_tx.send(SocketEvent::ForceDisconnect).is_err() {
            warn!("failed to send socket event");
        }
    }

    fn message_received(&mut self, message: Message) {
        if !Payload::irn_tag_in_range(message.tag) {
            warn!("\ntag={} skip handling", message.tag);
            return;
        }
        trace!("decoding {}", message.id);
        match self
            .cipher
            .decode::<Payload>(&message.topic, &message.message)
        {
            Ok(Payload::Request(req)) => {
                let rpc: RpcRequest = RpcRequest {
                    topic: message.topic,
                    payload: req,
                };
                if let Err(e) = self.req_tx.send(rpc) {
                    warn!("[relay handler] request channel is broken, error: {e}");
                }
            }
            Ok(Payload::Response(res)) => {
                if let Err(e) = self.res_tx.send(res) {
                    warn!("[relay handler] response channel is broken, error: {e}");
                }
            }
            Err(err) => {
                error!("failed to decode message id {} ({err})", message.id);
            }
        }
    }

    fn inbound_error(&mut self, _error: ClientError) {
        self.disconnected(None);
    }

    fn outbound_error(&mut self, _error: ClientError) {
        self.disconnected(None);
    }
}

async fn event_loop_request(
    mut rx: mpsc::UnboundedReceiver<RpcRequest>,
    actor: Address<RequestHandlerActor>,
) {
    info!("started event loop for requests");
    while let Some(req) = rx.recv().await {
        if let Err(err) = actor.send(req).await {
            error!("request actor shutdown! {err}");
            return;
        }
    }
}

async fn event_loop_response(
    mut rx: mpsc::UnboundedReceiver<Response>,
    actor: Address<InboundResponseActor>,
) {
    info!("started event loop for response");
    while let Some(r) = rx.recv().await {
        if let Err(e) = actor.send(r).await {
            warn!("actor channel has closed: {e}");
            return;
        }
    }
}
