use crate::actors::{Actors, InboundResponseActor, RequestHandlerActor, SocketActor};
use crate::domain::Message;
use crate::relay::{ClientError, CloseFrame, ConnectionHandler};
use crate::rpc::{Payload, Response, RpcRequest};
use crate::{Cipher, SocketEvent};
use tokio::sync::mpsc;
use tracing::{error, info, trace, warn};
use xtra::prelude::*;

pub struct RelayHandler {
    cipher: Cipher,
    req_tx: mpsc::UnboundedSender<RpcRequest>,
    res_tx: mpsc::UnboundedSender<Response>,
    socket_tx: mpsc::UnboundedSender<SocketEvent>,
}

impl RelayHandler {
    pub(crate) fn new(cipher: Cipher, actors: Actors) -> Self {
        let (req_tx, req_rx) = mpsc::unbounded_channel::<RpcRequest>();
        let (res_tx, res_rx) = mpsc::unbounded_channel::<Response>();
        let (socket_tx, socket_rx) = mpsc::unbounded_channel::<SocketEvent>();
        let req_actor = actors.request();
        let res_actor = actors.response();
        tokio::spawn(async move {
            event_loop_req(req_rx, req_actor).await;
        });
        tokio::spawn(async move {
            event_loop_res(res_rx, res_actor).await;
        });

        tokio::spawn(async move {
            event_loop_socket(socket_rx, actors.sockets()).await;
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
        if let Err(_) = self.socket_tx.send(SocketEvent::Connected) {
            warn!("failed to send socket event");
        }
    }

    fn disconnected(&mut self, _frame: Option<CloseFrame<'static>>) {
        if let Err(_) = self.socket_tx.send(SocketEvent::ForceDisconnect) {
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

async fn event_loop_socket(
    mut rx: mpsc::UnboundedReceiver<SocketEvent>,
    actor: Address<SocketActor>,
) {
    info!("started event loop for sockets");
    while let Some(r) = rx.recv().await {
        if let Err(_) = actor.send(r).await {
            warn!("[socket] actor channel has closed");
            return;
        }
    }
}

async fn event_loop_req(
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

async fn event_loop_res(
    mut rx: mpsc::UnboundedReceiver<Response>,
    actor: Address<InboundResponseActor>,
) {
    info!("started event loop for response");
    while let Some(r) = rx.recv().await {
        if let Err(_) = actor.send(r).await {
            warn!("actor channel has closed");
            return;
        }
    }
}
