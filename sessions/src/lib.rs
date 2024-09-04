mod actors;
pub mod crypto;
mod dapp;
mod domain;
mod error;
mod pair;
pub mod pairing_uri;
mod relay;
pub mod rpc;
pub mod session;
mod storage;
mod transport;
mod wallet;

pub use crate::session::ClientSession;
use async_trait::async_trait;
pub use crypto::cipher::Cipher;
pub use dapp::Dapp;
pub use domain::Message;
pub use error::Error;
pub use pair::{PairingManager, WalletConnectBuilder};
pub use pairing_uri::Pairing;
use pin_project_lite::pin_project;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Once};
use std::task::{Context, Poll};
use std::time::Duration;
pub use storage::KvStorage;
use tokio::sync::oneshot;
pub use wallet::Wallet;
pub type Atomic<T> = Arc<Mutex<T>>;
use crate::rpc::{SessionDeleteRequest, SessionRequestRequest};
pub use actors::{Actors, RegisteredManagers};
pub use domain::*;
pub use rpc::{Metadata, SdkErrors};
use walletconnect_namespaces::Event;
pub use walletconnect_relay::ClientError;
pub type PairingTopic = Topic;
pub type SessionTopic = Topic;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum SocketEvent {
    Connected,
    #[default]
    Disconnect,
    ForceDisconnect,
}

impl Display for SocketEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connected => {
                write!(f, "connected")
            }
            Self::Disconnect => {
                write!(f, "disconnected")
            }
            Self::ForceDisconnect => {
                write!(f, "force disconnect")
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
#[allow(dead_code)]
static INIT: Once = Once::new();

pin_project! {
    pub struct ProposeFuture<T> {
        #[pin]
        receiver: oneshot::Receiver<T>,
    }
}

impl<T> ProposeFuture<T> {
    #[must_use]
    pub fn new(receiver: oneshot::Receiver<T>) -> Self {
        Self { receiver }
    }
}

impl<T> Future for ProposeFuture<T> {
    type Output = Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().receiver.poll(cx) {
            Poll::Ready(Ok(value)) => Poll::Ready(Ok(value)),
            Poll::Ready(Err(_)) => Poll::Ready(Err(Error::ReceiveError)),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub enum SessionEvent {
    Event(Event),
    Request(SessionRequestRequest),
}

#[async_trait]
pub trait SessionEventHandler: Send + Sync + 'static {
    async fn event(&self, event: Event);
}

#[async_trait]
pub trait SessionHandlers: Send + Sync + 'static + SessionEventHandler {
    async fn request(&self, request: SessionRequestRequest);
}

pub struct NoopSessionHandler;

#[async_trait]
impl SessionEventHandler for NoopSessionHandler {
    async fn event(&self, event: Event) {
        tracing::info!("got session event {event:#?}");
    }
}

#[async_trait]
impl SessionHandlers for NoopSessionHandler {
    async fn request(&self, request: SessionRequestRequest) {
        tracing::info!("got session request");
    }
}

/*
#[trait_variant::make(Send + Sync)]
pub trait SessionDeleteHandler {
    async fn handle(&self, _request: SessionDeleteRequest);
}
 */

pub struct NoopSessionDeleteHandler;
impl SessionDeleteHandler for NoopSessionDeleteHandler {}

#[async_trait]
pub trait SessionDeleteHandler: Send + Sync + 'static {
    async fn handle(&self, request: SessionDeleteRequest) {
        tracing::info!("Session delete request {:#?}", request);
    }
}

pub(crate) fn shorten_topic(id: &Topic) -> String {
    let mut id = format!("{id}");
    if id.len() > 10 {
        id = String::from(&id[0..9]);
    }
    id
}

#[cfg(test)]
pub(crate) mod test {
    use crate::{NoopSessionHandler, SessionHandlers, INIT};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::EnvFilter;
    use walletconnect_namespaces::Event;
    use xtra::prelude::*;

    pub(crate) fn init_tracing() {
        INIT.call_once(|| {
            tracing_subscriber::fmt()
                .with_target(true)
                .with_level(true)
                .with_span_events(FmtSpan::CLOSE)
                .with_env_filter(EnvFilter::from_default_env())
                .init();
        });
    }

    #[derive(Clone, Actor)]
    struct Actor1 {
        handlers: Arc<Mutex<Vec<Box<dyn SessionHandlers>>>>,
    }

    #[derive(Actor)]
    struct Actor2 {}

    #[derive(Clone)]
    struct Dummy;

    impl Handler<Box<dyn SessionHandlers>> for Actor1 {
        type Return = ();

        async fn handle(
            &mut self,
            message: Box<dyn SessionHandlers>,
            ctx: &mut Context<Self>,
        ) -> Self::Return {
            self.handlers.lock().await.push(message);
        }
    }

    impl Actor1 {
        async fn handle_event(&self, event: Event) {
            let l = self.handlers.lock().await;
            for h in l.iter() {
                h.event(event.clone()).await;
            }
        }
    }

    impl Handler<Event> for Actor1 {
        type Return = ();

        async fn handle(&mut self, message: Event, ctx: &mut Context<Self>) -> Self::Return {
            let me = self.clone();
            tokio::spawn(async move {
                me.handle_event(message).await;
            });
        }
    }

    impl Handler<Dummy> for Actor1 {
        type Return = ();

        async fn handle(&mut self, message: Dummy, ctx: &mut Context<Self>) -> Self::Return {
            tracing::info!("Actor1 got message");
        }
    }

    impl Handler<Dummy> for Actor2 {
        type Return = ();

        async fn handle(&mut self, message: Dummy, ctx: &mut Context<Self>) -> Self::Return {
            tracing::info!("Actor2 got message");
        }
    }

    #[tokio::test]
    async fn test_actor_broadcast() -> anyhow::Result<()> {
        init_tracing();
        let handlers: Arc<Mutex<Vec<Box<dyn SessionHandlers>>>> =
            Arc::new(Mutex::new(vec![Box::new(NoopSessionHandler {})]));
        let boxed: Box<dyn SessionHandlers> = Box::new(NoopSessionHandler {});
        let act = Actor1 { handlers };
        let a1 = xtra::spawn_tokio(act.clone(), Mailbox::unbounded());
        a1.send(Dummy).await?;
        a1.send(boxed).await?;
        a1.send(Event::AccountsChanged).await?;
        //a2.broadcast(Dummy).await?;
        tokio::time::sleep(Duration::from_secs(3)).await;
        eprintln!("size {}", act.handlers.lock().await.len());
        Ok(())
    }
}
