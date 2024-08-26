use crate::actors::{CipherActor, DeleteSession};
use crate::rpc::{RequestParams, SessionDeleteRequest, SettleNamespaces};
use crate::transport::SessionTransport;
use crate::Topic;
use crate::{Result, SessionDeleteHandler};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, warn};
use xtra::prelude::*;

mod session_delete;
mod session_ping;

/// https://specs.walletconnect.com/2.0/specs/clients/sign/session-proposal
///
/// New session as the result of successful session proposal.
#[derive(Clone, Actor)]
pub struct ClientSession {
    pub namespaces: Arc<SettleNamespaces>,
    transport: SessionTransport,
    cipher_actor: Address<CipherActor>,
    delete_sender: mpsc::Sender<SessionDeleteRequest>,
}

impl ClientSession {
    pub(crate) fn new<T: SessionDeleteHandler>(
        cipher_actor: Address<CipherActor>,
        transport: SessionTransport,
        namespaces: SettleNamespaces,
        handler_delete: T,
    ) -> Self {
        let (delete_sender, delete_receiver) = mpsc::channel(32);
        tokio::spawn(async move {
            session_delete::handle_delete(handler_delete, delete_receiver).await;
        });
        Self {
            transport,
            namespaces: Arc::new(namespaces),
            delete_sender,
            cipher_actor,
        }
    }
}

impl ClientSession {
    pub fn namespaces(&self) -> &SettleNamespaces {
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
        self.cipher_actor
            .send(DeleteSession(self.topic()))
            .await??;
        Ok(())
    }
}
