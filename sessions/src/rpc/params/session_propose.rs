//! https://specs.walletconnect.com/2.0/specs/clients/sign/rpc-methods
//! #wc_sessionpropose

use std::fmt::{Debug, Display, Formatter};

use monedero_namespaces::Namespaces;
use serde::{Deserialize, Serialize};

use super::{IrnMetadata, Metadata, RelayProtocol};
use crate::rpc::{ErrorParams, IntoUnknownError, ResponseParamsError};

pub(super) const IRN_REQUEST_METADATA: IrnMetadata = IrnMetadata {
    tag: 1100,
    ttl: 300,
    prompt: true,
};

pub(super) const IRN_RESPONSE_METADATA: IrnMetadata = IrnMetadata {
    tag: 1101,
    ttl: 300,
    prompt: false,
};

#[derive(Debug, Serialize, Eq, PartialEq, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Proposer {
    pub public_key: String,
    pub metadata: Metadata,
}

impl Proposer {
    #[must_use]
    pub fn new(key: String, metadata: Metadata) -> Self {
        Self {
            public_key: key,
            metadata,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionProposeRequest {
    pub relays: Vec<RelayProtocol>,
    pub proposer: Proposer,
    pub required_namespaces: Namespaces,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub optional_namespaces: Option<Namespaces>,
}

impl Display for SessionProposeRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let opt = self
            .optional_namespaces
            .as_ref()
            .map_or_else(String::new, |o| format!("{o}"));
        /*
        let opt = match &self.optional_namespaces {
            Some(o) => format!("{}", o),
            None => String::new(),
        };
             */
        write!(
            f,
            "required:[{}] optional:[{}]",
            self.required_namespaces, opt
        )
    }
}

impl SessionProposeRequest {
    pub fn new(
        metadata: Metadata,
        public_key: String,
        required: Namespaces,
        optional: Option<Namespaces>,
    ) -> Self {
        Self {
            relays: vec![RelayProtocol::default()],
            proposer: Proposer {
                public_key,
                metadata,
            },
            required_namespaces: required,
            optional_namespaces: optional,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionProposeResponse {
    pub relay: RelayProtocol,
    pub responder_public_key: String,
}

impl IntoUnknownError for SessionProposeRequest {
    fn unknown(&self) -> ResponseParamsError {
        ResponseParamsError::SessionPropose(ErrorParams::unknown())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::super::tests::param_serde_test;
    use super::*;

    #[test]
    fn test_serde_session_propose_request() -> Result<()> {
        // https://specs.walletconnect.com/2.0/specs/clients/sign/
        // session-events#session_propose
        let json = r#"
        {
            "relays": [
                {
                    "protocol": "irn"
                }
            ],
            "proposer": {
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
            "requiredNamespaces": {
                "eip155": {
                    "chains": [
                        "eip155:5"
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
            }
        }
        "#;

        param_serde_test::<SessionProposeRequest>(json)
    }
}
