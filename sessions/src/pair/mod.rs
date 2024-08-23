mod builder;
//pub(crate) mod settlement;

use std::future::Future;
use crate::domain::{SubscriptionId, Topic};
use crate::relay::{Client, ConnectionHandler, ConnectionOptions};
use crate::rpc::{ErrorParams, Metadata, PairPingRequest, ProposeNamespaces, Proposer, RelayProtocol, Request, RequestParams, Response, ResponseParams, ResponseParamsSuccess, RpcResponse, SessionProposeRequest};
use crate::session::{ClientSession, RelayHandler};
use crate::transport::{PendingRequests, RpcRecv, TopicTransport};
use crate::{relay, session, Cipher, EventChannel, KvStorage, Pairing, Result, WireEvent};
pub use builder::WalletConnectBuilder;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use serde_json::json;
use tokio::sync::{broadcast, oneshot};
use tracing::{info, warn};
use xtra::{Context, Handler, Mailbox};
use crate::actors::Actors;

pub trait PairHandler: Send + 'static {
    fn ping(&mut self, topic: Topic);
    fn delete(&mut self, reason: ErrorParams, topic: Topic);
}

#[derive(Clone, xtra::Actor)]
pub struct PairingManager {
    relay: Client,
    opts: ConnectionOptions,
    ciphers: Cipher,
    metadata: Metadata,
    transport: TopicTransport,
    terminator: broadcast::Sender<()>,
    //storage: Arc<KvStorage>,
    actors: Actors
}

impl Handler<PairPingRequest> for PairingManager {
    type Return = ResponseParams;

    async fn handle(&mut self, _message: PairPingRequest, _ctx: &mut Context<Self>) -> Self::Return {
        ResponseParamsSuccess::PairPing(true).try_into().unwrap()
    }
}

async fn handle_socket_close(mgr: PairingManager, mut rx: broadcast::Receiver<WireEvent>) {
    loop {
        match rx.recv().await {
            Ok(_) => {
                tracing::info!("reconnecting after 5 seconds");
                tokio::time::sleep(Duration::from_secs(5)).await;
                if let Err(e) = mgr.socket_open().await {
                    // backoff
                    tracing::error!("failed to reconnect {e}");
                }
            }
            Err(_) => {
                return;
            }
        }
    }
}

impl PairingManager {
    async fn init(metadata: Metadata, opts: ConnectionOptions, actors: Actors) -> Result<Self> {
        //let (broadcast_tx, _broadcast_rx) = broadcast::channel::<WireEvent>(5);
        let (terminator, terminate_rx) = broadcast::channel::<()>(2);
        //let pending_requests = PendingRequests::new();
        //let storage = Arc::new(storage);
        let ciphers = actors.cipher().clone();
        //let socket_handler_rx = broadcast_tx.subscribe();
        let handler = RelayHandler::new(
            ciphers.clone(),
            actors.clone()
        );
        #[cfg(feature = "mock")]
        let relay = Client::mock(handler);

        #[cfg(not(feature = "mock"))]
        let relay = Client::new(handler);

        actors.register_client(relay.clone()).await?;
        relay.connect(&opts).await?;

        let transport = TopicTransport::new(actors.transport());

        //tokio::spawn(session::handle_session_request(broadcast_rx, terminate_rx));
        let mgr = Self {
            relay,
            opts,
            ciphers,
            metadata,
            transport,
            terminator,
            //storage,
            actors
        };
        //let socker_handler = mgr.clone();
        //tokio::spawn(async move { handle_socket_close(socker_handler, socket_handler_rx).await });
        mgr.socket_open().await?;
        Ok(mgr)
    }

    pub fn ciphers(&self) -> Cipher {
        self.ciphers.clone()
    }

    pub(crate) async fn subscribe(&self, topic: Topic) -> Result<(SubscriptionId)> {
        Ok(self.relay.subscribe(topic).await?)
    }

    /*
    pub fn storage(&self) -> Arc<KvStorage> {
        self.storage.clone()
    }
     */

    pub(crate) fn actors(&self) -> Actors {
        self.actors.clone()
    }

    pub fn pair_key(&self) -> Option<String> {
        self.ciphers.public_key_hex()
    }

    pub async fn propose(
        &self,
        namespaces: ProposeNamespaces,
    ) -> Result<(Arc<Pairing>, crate::EventClientSession)> {
        let cipher = self.ciphers.clone();
        let pairing: Pairing = Default::default();
        cipher.set_pairing(Some(pairing.clone()))?;
        let pairing = Arc::new(pairing);
        self.relay.subscribe(pairing.topic.clone()).await?;
        let key = match self.ciphers.public_key_hex() {
            None => return Err(crate::error::Error::PairingInitError),
            Some(k) => k,
        };
        let payload = RequestParams::SessionPropose(SessionProposeRequest {
            relays: vec![RelayProtocol::default()],
            proposer: Proposer::new(key, self.metadata.clone()),
            required_namespaces: namespaces,
        });

        let (tx, rx) = oneshot::channel::<Result<ClientSession>>();
        /*
        tokio::spawn(settlement::process_settlement(
            self.clone(),
            tx,
            self.broadcast_tx.clone(),
            pairing.clone(),
            payload,
        ));
         */
        Ok((pairing, rx))
    }

    pub fn topic(&self) -> Option<Topic> {
        self.ciphers.pairing().map(|p| p.topic.clone())
    }

    pub async fn ping(&self) -> Result<()> {
        let t = self.topic().ok_or(crate::Error::NoPairingTopic)?;
        self.transport
            .publish_request(t, RequestParams::PairPing(()))
            .await
    }

    pub async fn shutdown(&self) -> Result<()> {
        //self.broadcast_tx.send(WireEvent::Shutdown).unwrap();
        self.socket_disconnect().await
    }

    pub async fn socket_open(&self) -> Result<()> {
        info!("opening websocket");
        self.relay.connect(&self.opts).await?;
        Ok(())
    }

    pub async fn socket_disconnect(&self) -> Result<()> {
        info!("closing websocket");
        if let Err(err) = self.relay.disconnect().await {
            warn!("failed to close socket {err}");
        }
        //self.broadcast_tx.send(WireEvent::Disconnect).map_err(|_| crate::Error::LockError)?;
        Ok(())
    }

    //pub fn event_subscription(&self) -> EventChannel {
        //self.broadcast_tx.subscribe()
    //}
}

/*
#[cfg(feature = "mock")]
#[cfg(test)]
mod test {
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use crate::{Atomic, EventChannel};
    use crate::domain::ProjectId;
    use super::*;
    use crate::relay::mock::test::auth;

    #[derive(Clone)]
    struct EventHistory {
        events: Atomic<VecDeque<WireEvent>>
    }

    async fn event_history(events: EventHistory, mut rx: EventChannel) {
        loop {
            if let Ok(result) = rx.recv().await {
                if let Ok(mut lock) = events.events.lock() {
                    match result {
                        WireEvent::Shutdown => {
                            lock.push_front(WireEvent::Shutdown);
                            return
                        }
                        _ => {}
                    }
                    lock.push_back(result);

                }
            }
        }
    }

    #[tokio::test]
    async fn manager_test() -> anyhow::Result<()> {
        let p = ProjectId::from("9d5b20b16777cc49100cf9df3649bd24");
        let auth_token = auth();

        let mgr = WalletConnectBuilder::new(p, auth_token).build().await?;
        let event_channel = mgr.event_subscription();
        let history = EventHistory{
            events: Arc::new(Mutex::new(Default::default()))
        };
        let hist = history.clone();
        tokio::spawn(async move {
            event_history(hist, event_channel).await
        });

        mgr.socket_disconnect().await?;
        mgr.shutdown().await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        let lock = history.events.lock().unwrap();
        assert_eq!(2, lock.len());
        Ok(())
    }
}

 */