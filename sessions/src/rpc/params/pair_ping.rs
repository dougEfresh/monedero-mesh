//! https://specs.walletconnect.com/2.0/specs/clients/core/pairing/rpc-methods
//! #wc_pairingPing

use super::IrnMetadata;
use crate::rpc::{TAG_PAIR_PING_REQUEST, TAG_PAIR_PING_RESPONSE};
use serde::{Deserialize, Serialize};

pub(super) const IRN_REQUEST_METADATA: IrnMetadata = IrnMetadata {
    tag: TAG_PAIR_PING_REQUEST,
    ttl: 30,
    prompt: false,
};

pub(super) const IRN_RESPONSE_METADATA: IrnMetadata = IrnMetadata {
    tag: TAG_PAIR_PING_RESPONSE,
    ttl: 30,
    prompt: false,
};

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PairPingRequest {}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::tests::param_serde_test;
    use anyhow::Result;

    #[test]
    fn test_serde_pair_ping_request() -> Result<()> {
        let json = r#"{}"#;

        param_serde_test::<PairPingRequest>(json)
    }
}
