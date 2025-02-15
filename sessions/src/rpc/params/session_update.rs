//! (wc_sessionUpdate)[https://specs.walletconnect.com/2.0/specs/clients/sign/rpc-methods#wc_sessionupdate]

use {
    super::IrnMetadata,
    monedero_domain::namespaces::Namespaces,
    serde::{Deserialize, Serialize},
};

pub(super) const IRN_REQUEST_METADATA: IrnMetadata = IrnMetadata {
    tag: 1104,
    ttl: 86400,
    prompt: false,
};

pub(super) const IRN_RESPONSE_METADATA: IrnMetadata = IrnMetadata {
    tag: 1105,
    ttl: 86400,
    prompt: false,
};

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionUpdateRequest {
    pub namespaces: Namespaces,
}
