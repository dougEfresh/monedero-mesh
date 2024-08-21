use crate::domain::Message;
use crate::relay::{ClientError, CloseFrame, ConnectionHandler};
use crate::rpc::{Payload, Request, RequestParams, Response, ResponseParams};
use crate::transport::{PendingRequests, RpcRecv};
use crate::{Cipher, WireEvent};
use std::sync::Arc;
use std::thread::spawn;
use tokio::select;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};
use xtra::{Actor, Address};
use crate::actors::{RequestActor, ResponseActor};

pub struct RelayActHandler {
    cipher: Cipher,
    req_tx: mpsc::UnboundedSender<Request>,
    res_tx: mpsc::UnboundedSender<Response>,
}

impl RelayActHandler {
    pub fn new(cipher: Cipher, res_actor: Address<ResponseActor>, req_actor: Address<RequestActor>) -> Self {
        let (req_tx, req_rx) = mpsc::unbounded_channel::<Request>();
        let (res_tx, res_rx) = mpsc::unbounded_channel::<Response>();
        tokio::spawn(async move  {
            event_loop_req(req_rx, req_actor).await;
        });
        tokio::spawn(async move {
            event_loop_res(res_rx, res_actor).await;
        });
        Self {
            cipher,
            req_tx,
            res_tx
        }
    }
}

impl ConnectionHandler for RelayActHandler {
    fn message_received(&mut self, message: Message) {
        if !Payload::irn_tag_in_range(message.tag) {
            warn!("\ntag={} skip handling", message.tag);
            return;
        }
        debug!("decoding {}", message.id);
        match self
          .cipher
          .decode::<Payload>(&message.topic, &message.message) {
            Ok(Payload::Request(req)) => {
                self.req_tx.send(req).unwrap();
            }
            Ok(Payload::Response(res)) => {
                self.res_tx.send(res).unwrap();
            }
            Err(err) => {
                error!("failed to decode message id {}", message.id);
            }
        }
    }
}

pub struct RelayHandler {
    cipher: Cipher,
    // pending_requests: PendingRequests,
     //tx: mpsc::Sender<Payload>,
}

async fn event_loop_req(mut rx: mpsc::UnboundedReceiver<Request>, _actor: Address<RequestActor>)  {
    info!("started event loop for requests");
    while let Some(r) = rx.recv().await {
        debug!("request");
    }
}

async fn event_loop_res(mut rx: mpsc::UnboundedReceiver<Response>, actor: Address<ResponseActor>)  {
    info!("started event loop for requests");
    while let Some(r) = rx.recv().await {
        if let Err(_) = actor.send(r).await {
            warn!("actor channel has closed");
            return;
        }
    }
}

impl RelayHandler {
    pub(crate) fn new(
        tx: broadcast::Sender<WireEvent>,
        pending_requests: PendingRequests,
        cipher: Cipher,
    ) -> Self {
        Self {
          //  tx,
            cipher,
      //      pending_requests,
        }
    }
}

impl ConnectionHandler for RelayHandler {
    fn connected(&mut self) {
        //crate::send_event(&self.tx, WireEvent::Connected);
    }

    fn disconnected(&mut self, _frame: Option<CloseFrame<'static>>) {
        //crate::send_event(&self.tx, WireEvent::DisconnectFromHandler);
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn message_received(&mut self, message: Message) {
        if !Payload::irn_tag_in_range(message.tag) {
            warn!("\ntag={} skip handling", message.tag);
            return;
        }
        debug!("decoding {}", message.id);
        /*
        match self
          .cipher
          .decode::<Payload>(&message.topic, &message.message) {
            Ok(Payload::Request(req)) => {
                // send payload to actor
            }
            Ok(Payload::Response(res)) => {
                // send payload to actor
            }
            Err(err) => {
                error!("failed to decode message id {}", message.id);
            }
        }
         */
    }
    /*
    #[tracing::instrument(level = "trace", skip(self))]
    fn message_received(&mut self, message: Message) {
            if !Payload::irn_tag_in_range(message.tag) {
            warn!("\ntag={} skip handling", message.tag);
            return;
        }
        crate::send_event(&self.tx, WireEvent::MessageRecv(message.clone()));
        match self
            .cipher
            .decode::<Payload>(&message.topic, &message.message)
        {
            Ok(payload) => {
                match payload {
                    // The other side is responding to a request made from us
                    Payload::Response(res) => {
                        self.pending_requests
                            .handle_response(&res.id, res.params.clone());
                        let rpc: RpcRecv<ResponseParams> =
                            RpcRecv::new(message.id, message.topic, res.params);
                        let event: WireEvent = WireEvent::ResponseRecv(rpc);
                        crate::send_event(&self.tx, event);
                    }
                    Payload::Request(req) => {
                        let rpc: RpcRecv<RequestParams> =
                            RpcRecv::new(req.id, message.topic, req.params);
                        let event: WireEvent = WireEvent::RequestRecv(rpc);
                        crate::send_event(&self.tx, event);
                    }
                }
            }
            Err(e) => {
                crate::send_event(
                    &self.tx,
                    WireEvent::DecryptError(message, format!("{e}").into()),
                );
                return;
            }
        };
    }

     */

    fn inbound_error(&mut self, error: ClientError) {
        warn!("[inbound] Connection was closed inbound error: {error}");
        //crate::send_event(&self.tx, WireEvent::DisconnectFromHandler);
    }

    fn outbound_error(&mut self, error: ClientError) {
        warn!("[outbound] Connection appears down: {error}");
        //crate::send_event(&self.tx, WireEvent::DisconnectFromHandler);
    }
}


#[cfg(test)]
mod test {
    use std::sync::Arc;
    use std::time::Duration;
    use anyhow::format_err;
    use serde_json::json;
    use xtra::Mailbox;
    use crate::actors::AddRequest;
    use crate::domain::SubscriptionId;
    use crate::rpc;
    use super::*;

    #[tokio::test]
    async fn test_relay() -> anyhow::Result<()> {
        let (dapp, wallet) = crate::tests::dapp_wallet_ciphers()?;
        let pairing = dapp.pairing().ok_or(Err(format_err!("no pairing!")))?;
        let topic = pairing.topic.clone();

        let res_addr = xtra::spawn_tokio(ResponseActor::default(), Mailbox::unbounded());
        let req_addr = xtra::spawn_tokio(RequestActor::default(), Mailbox::unbounded());
        let (id, rx) = res_addr.send(AddRequest).await?;
        let resp = Response::new(id.clone(), ResponseParams::Success(json!(true)));
        //addr.send(resp)
        let mut handler = RelayActHandler::new(dapp, res_addr.clone(), req_addr.clone());
        let payload = wallet.encode(&topic,&resp)?;

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
        Ok(())
    }
}