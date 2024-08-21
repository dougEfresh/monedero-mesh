//! https://specs.walletconnect.com/2.0/specs/clients/sign/data-structures

mod propose_namespaces;
mod settle_namespaces;

use crate::rpc::RELAY_PROTOCOL;
use serde::{Deserialize, Serialize};
pub use {
    propose_namespaces::{ProposeNamespace, ProposeNamespaceError, ProposeNamespaces},
    settle_namespaces::{SettleNamespace, SettleNamespaces},
};

/// The maximum number of topics allowed for a batch subscribe request.
///
/// See <https://github.com/WalletConnect/walletconnect-docs/blob/main/docs/specs/servers/relay/relay-server-rpc.md>
pub const MAX_SUBSCRIPTION_BATCH_SIZE: usize = 500;

/// The maximum number of topics allowed for a batch fetch request.
///
/// See <https://github.com/WalletConnect/walletconnect-docs/blob/main/docs/specs/servers/relay/relay-server-rpc.md>
pub const MAX_FETCH_BATCH_SIZE: usize = 500;

/// The maximum number of receipts allowed for a batch receive request.
///
/// See <https://github.com/WalletConnect/walletconnect-docs/blob/main/docs/specs/servers/relay/relay-server-rpc.md>
pub const MAX_RECEIVE_BATCH_SIZE: usize = 500;

pub const TAG_SESSION_PROPOSE_REQUEST: u32 = 1100;
pub const TAG_SESSION_PROPOSE_RESPONSE: u32 = 1101;

pub const TAG_SESSION_SETTLE_REQUEST: u32 = 1102;
pub const TAG_SESSION_SETTLE_RESPONSE: u32 = 1103;

pub const TAG_SESSION_UPDATE_REQUEST: u32 = 1104;
pub const TAG_SESSION_UPDATE_RESPONSE: u32 = 1105;

pub const TAG_SESSION_EXTEND_REQUEST: u32 = 1106;
pub const TAG_SESSION_EXTEND_RESPONSE: u32 = 1107;

pub const TAG_SESSION_REQUEST_REQUEST: u32 = 1108;
pub const TAG_SESSION_REQUEST_RESPONSE: u32 = 1109;

pub const TAG_SESSION_EVENT_REQUEST: u32 = 1110;
pub const TAG_SESSION_EVENT_RESPONSE: u32 = 1111;

pub const TAG_SESSION_DELETE_REQUEST: u32 = 1112;
pub const TAG_SESSION_DELETE_RESPONSE: u32 = 1113;

pub const TAG_SESSION_PING_REQUEST: u32 = 1114;
pub const TAG_SESSION_PING_RESPONSE: u32 = 1115;

pub const TAG_PAIR_DELETE_REQUEST: u32 = 1000;
pub const TAG_PAIR_DELETE_RESPONSE: u32 = 1001;

pub const TAG_PAIR_PING_REQUEST: u32 = 1002;
pub const TAG_PAIR_PING_RESPONSE: u32 = 1003;

pub const TAG_PAIR_EXTEND_REQUEST: u32 = 1004;
pub const TAG_PAIR_EXTEND_RESPONSE: u32 = 1005;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Redirects {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub universal: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub name: String,
    pub description: String,
    pub url: String,
    pub icons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect: Option<Redirects>,
}

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone)]
pub struct RelayProtocol {
    pub protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub data: Option<String>,
}

impl Default for RelayProtocol {
    fn default() -> Self {
        Self {
            protocol: String::from(RELAY_PROTOCOL),
            data: None,
        }
    }
}
