use crate::actors::{Actors, InboundResponseActor, RequestHandlerActor, SocketActors};
use crate::domain::Message;
use crate::relay::{ClientError, CloseFrame, ConnectionHandler};
use crate::rpc::{Payload, Request, RequestParams, Response, ResponseParams, RpcRequest};
use crate::transport::{PendingRequests, RpcRecv};
use crate::{Cipher, SocketEvent, WireEvent};
use std::sync::Arc;
use std::thread::spawn;
use tokio::select;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};
use xtra::{Actor, Address};

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
        debug!("decoding {}", message.id);
        match self
            .cipher
            .decode::<Payload>(&message.topic, &message.message)
        {
            Ok(Payload::Request(req)) => {
                let rpc: RpcRequest = RpcRequest {
                    topic: message.topic,
                    payload: req,
                };
                self.req_tx.send(rpc).unwrap();
            }
            Ok(Payload::Response(res)) => {
                self.res_tx.send(res).unwrap();
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
    actor: Address<SocketActors>,
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

#[cfg(feature = "mock")]
#[cfg(test)]
mod test {
    use super::*;
    use crate::tests::{yield_ms, ConnectionState};

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn test_relay_connection_state() -> anyhow::Result<()> {
        let test_components = crate::tests::init_test_components(false).await?;
        let actors = test_components.dapp_actors;
        let dapp_cipher = test_components.dapp_cipher;
        let socket_state = ConnectionState::default();
        actors.register_socket_handler(socket_state.clone()).await?;

        let mut handler = RelayHandler::new(dapp_cipher, actors);
        yield_ms(500).await;
        assert_eq!(SocketEvent::Disconnect, socket_state.get_state());
        handler.connected();
        yield_ms(500).await;
        assert_eq!(SocketEvent::Connected, socket_state.get_state());
        handler.disconnected(None);
        yield_ms(500).await;
        assert_eq!(SocketEvent::ForceDisconnect, socket_state.get_state());

        handler.connected();
        yield_ms(500).await;
        handler.inbound_error(ClientError::Disconnected);
        yield_ms(500).await;
        assert_eq!(SocketEvent::ForceDisconnect, socket_state.get_state());

        handler.connected();
        yield_ms(500).await;
        handler.outbound_error(ClientError::Disconnected);
        yield_ms(500).await;
        assert_eq!(SocketEvent::ForceDisconnect, socket_state.get_state());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_relay_request() -> anyhow::Result<()> {
        let test_components = crate::tests::init_test_components(true).await?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_relay_response() -> anyhow::Result<()> {
        let test_components = crate::tests::init_test_components(true).await?;
        let dapp = test_components.dapp;
        dapp.ping().await?;
        /*
        let dapp_cipher = test_components.dapp_cipher;
        let wallet_cipher = test_components.wallet_cipher;
        let actors = test_components.actors;
        let pairing = dapp_cipher.pairing().ok_or(format_err!("no pairing!"))?;
        let topic = pairing.topic.clone();
        let res_actor = actors.response();
        let (id, rx) = res_actor.send(AddRequest).await?;
        let resp = Response::new(id.clone(), ResponseParams::Success(json!(true)));
        let mut handler = RelayHandler::new(dapp_cipher, actors);
        yield_ms(500).await;
        let payload = wallet_cipher.encode(&topic, &resp)?;
        let msg = Message {
            id: id.clone(),
            subscription_id: SubscriptionId::generate(),
            topic: pairing.topic.clone(),
            message: Arc::from(payload.as_str()),
            tag: rpc::TAG_SESSION_PROPOSE_REQUEST,
            published_at: Default::default(),
            received_at: Default::default(),
        };
        handler.message_received(msg);
        let result = tokio::time::timeout(Duration::from_secs(1), rx).await??;
        let should_be_true: bool = serde_json::from_value(result?)?;
        assert!(should_be_true);
         */
        Ok(())
    }
}
