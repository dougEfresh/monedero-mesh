use crate::rpc::{RequestParams, SessionDeleteRequest};
use crate::transport::SessionTransport;
use crate::{Cipher, Error, PairingManager, PairingTopic, SessionEvent, SessionHandlers, Topic};
use crate::{Result, SessionDeleteHandler};
use dashmap::DashMap;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tokio::sync::oneshot::Sender;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, warn};
use walletconnect_namespaces::Namespaces;
use xtra::prelude::*;

mod session_delete;
mod session_ping;
mod pending;

pub(crate) use pending::PendingSession;


/// https://specs.walletconnect.com/2.0/specs/clients/sign/session-proposal
///
/// New session as the result of successful session proposal.
#[derive(Clone, Actor)]
pub struct ClientSession {
    pub namespaces: Arc<Namespaces>,
    transport: SessionTransport,
    cipher: Cipher,
    tx: mpsc::UnboundedSender<SessionEvent>,
}

impl ClientSession {
    pub(crate) fn new(
        cipher: Cipher,
        transport: SessionTransport,
        namespaces: Namespaces,
        tx: mpsc::UnboundedSender<SessionEvent>,
    ) -> Self {
        Self {
            transport,
            namespaces: Arc::new(namespaces),
            cipher,
            tx,
        }
    }
}

impl ClientSession {
    pub fn namespaces(&self) -> &Namespaces {
        &self.namespaces
    }

    pub fn topic(&self) -> Topic {
        self.transport.topic.clone()
    }

    pub async fn publish_request<R: DeserializeOwned>(&self, params: RequestParams) -> Result<R> {
        self.transport.publish_request(params).await
    }

    pub async fn ping(&self) -> Result<bool> {
        self.publish_request(RequestParams::SessionPing(())).await
    }

    pub async fn delete(&self) -> Result<bool> {
        let accepted: bool = match self
            .publish_request(RequestParams::SessionDelete(SessionDeleteRequest::default()))
            .await
        {
            Ok(false) => {
                warn!("other side did not accept our delete request");
                false
            }
            Ok(true) => true,
            Err(e) => {
                error!("failed send session delete: {e}");
                false
            }
        };
        self.cleanup_session().await?;
        Ok(accepted)
    }

    async fn cleanup_session(&self) -> Result<()> {
        self.transport.unsubscribe().await?;
        self.cipher.delete_session(&self.topic())?;
        Ok(())
    }
}
