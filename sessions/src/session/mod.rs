pub use crate::relay::relay_handler::RelayHandler;
use serde::de::DeserializeOwned;
use std::sync::Arc;

use crate::crypto::session::SessionKey;
use walletconnect_sdk::client::websocket::Client;
use walletconnect_sdk::rpc::domain::SubscriptionId;

use crate::rpc::{RequestParams, SettleNamespaces};
use crate::transport::SessionTransport;
use crate::Result;
use crate::Topic;

/// https://specs.walletconnect.com/2.0/specs/clients/sign/session-proposal
///
/// New session as the result of successful session proposal.
pub struct Session {
    /// Pairing subscription id.
    pub subscription_id: SubscriptionId,
    /// Session symmetric key.
    ///
    /// https://specs.walletconnect.com/2.0/specs/clients/core/crypto/
    /// crypto-keys#key-algorithms
    pub session_key: SessionKey,

    pub relay: Client,
}

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
