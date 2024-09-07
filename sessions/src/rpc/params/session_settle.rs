//! https://specs.walletconnect.com/2.0/specs/clients/sign/rpc-methods
//! #wc_sessionsettle

use std::fmt::{Display, Formatter};
use crate::rpc::params::Controller;
use crate::rpc::{ErrorParams, IntoUnknownError, ResponseParamsError};
use walletconnect_namespaces::Namespaces;
use {
    super::{IrnMetadata, RelayProtocol},
    serde::{Deserialize, Serialize},
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

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::tests::param_serde_test;
    use anyhow::Result;

    #[test]
    fn test_serde_session_settle_request() -> Result<()> {
        // Coppied from `session_propose` and adjusted slightly.
        let json = r#"
        {
            "relay": {
                "protocol": "irn"
            },
            "controller": {
                "publicKey": "a3ad5e26070ddb2809200c6f56e739333512015bceeadbb8ea1731c4c7ddb207",
                "metadata": {
                    "name": "React App",
                    "description": "React App for WalletConnect",
                    "url": "http://localhost:3000",
                    "icons": [
                        "https://avatars.githubusercontent.com/u/37784886"
                    ]
                }
            },
            "namespaces": {
                "eip155": {
                    "accounts": [
                        "eip155:5:0xBA5BA3955463ADcc7aa3E33bbdfb8A68e0933dD8"
                    ],
                    "methods": [
                        "eth_sendTransaction",
                        "eth_sign",
                        "eth_signTransaction",
                        "eth_signTypedData",
                        "personal_sign"
                    ],
                    "events": [
                        "accountsChanged",
                        "chainChanged"
                    ]
                }
            },
            "expiry": 1675734962
        }
        "#;

        param_serde_test::<SessionSettleRequest>(json)
    }
}
