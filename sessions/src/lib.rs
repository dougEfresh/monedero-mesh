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
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::sync::{Arc, Mutex, Once};
use std::time::Duration;
pub use storage::KvStorage;
pub use wallet::Wallet;
pub type Atomic<T> = Arc<Mutex<T>>;
use crate::rpc::SessionDeleteRequest;
pub use actors::{Actors, RegisteredManagers};
pub use domain::*;
pub use walletconnect_relay::ClientError;

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

use pin_project_lite::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::oneshot;

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
    use crate::INIT;
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::EnvFilter;

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
}
