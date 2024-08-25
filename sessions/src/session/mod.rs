use crate::rpc::{RequestParams, SettleNamespaces};
use crate::transport::SessionTransport;
use crate::Result;
use crate::Topic;
use serde::de::DeserializeOwned;
use std::sync::Arc;

/// https://specs.walletconnect.com/2.0/specs/clients/sign/session-proposal
///
/// New session as the result of successful session proposal.
#[derive(Clone, xtra::Actor)]
pub struct ClientSession {
    pub namespaces: Arc<SettleNamespaces>,
    transport: SessionTransport,
}

impl ClientSession {
    pub(crate) fn new(transport: SessionTransport, namespaces: SettleNamespaces) -> Self {
        Self {
            transport,
            namespaces: Arc::new(namespaces),
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
}
