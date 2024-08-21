mod relay_handler;

pub use relay_handler::RelayHandler;

use crate::crypto::session::SessionKey;
use walletconnect_sdk::client::websocket::Client;
use walletconnect_sdk::rpc::domain::SubscriptionId;

use crate::rpc::SettleNamespaces;
use crate::transport::SessionTransport;

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

pub struct ClientSession {
    pub namespaces: SettleNamespaces,
    transport: SessionTransport,
}

impl ClientSession {
    pub(crate) fn new(transport: SessionTransport, namespaces: SettleNamespaces) -> Self {
        Self {
            transport,
            namespaces,
        }
    }
}

impl ClientSession {
    pub fn namespaces(&self) -> &SettleNamespaces {
        &self.namespaces
    }
}
