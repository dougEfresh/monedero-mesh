//! https://specs.walletconnect.com/2.0/specs/clients/sign/rpc-methods
//! #wc_sessionsettle

use {
    super::{IrnMetadata, RelayProtocol},
    crate::rpc::{params::Controller, ErrorParams, IntoUnknownError, ResponseParamsError},
    monedero_domain::namespaces::Namespaces,
    serde::{Deserialize, Serialize},
    std::fmt::{Display, Formatter},
};

pub(super) const IRN_REQUEST_METADATA: IrnMetadata = IrnMetadata {
    tag: 1102,
    ttl: 300,
    prompt: false,
};

pub(super) const IRN_RESPONSE_METADATA: IrnMetadata = IrnMetadata {
    tag: 1103,
    ttl: 300,
    prompt: false,
};

#[derive(Debug, Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionSettleRequest {
    pub relay: RelayProtocol,
    pub controller: Controller,
    pub namespaces: Namespaces,
    /// Unix timestamp.
    ///
    /// Expiry should be between .now() + TTL.
    pub expiry: i64,
}

impl Display for SessionSettleRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "namespaces=[{}]", self.namespaces)
    }
}

impl IntoUnknownError for SessionSettleRequest {
    fn unknown(&self) -> ResponseParamsError {
        ResponseParamsError::SessionSettle(ErrorParams::unknown())
    }
}
