//! https://specs.walletconnect.com/2.0/specs/clients/core/pairing/pairing-uri

use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

use regex::Regex;
use serde::{Deserialize, Serialize};
use url::Url;
use x25519_dalek::StaticSecret;

use crate::crypto::cipher::DecodedSymKey;
use crate::domain::{DecodedTopic, Topic};
use crate::rpc::RELAY_PROTOCOL;

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum ParseError {
    #[error("Expecting protocol \"wc\" but \"{protocol}\" is found.")]
    UnexpectedProtocol { protocol: String },
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error("Failed to parse topic and version")]
    InvalidTopicAndVersion,
    #[error("Topic not found")]
    TopicNotFound,
    #[error("Version not found")]
    VersionNotFound,
    #[error("Relay protocol not found")]
    RelayProtocolNotFound,
    #[error("Key not found")]
    KeyNotFound,
    #[error("Failed to parse key: {0:?}")]
    InvalidKey(#[from] data_encoding::DecodeError),
    #[error("Invalid key length")]
    InvalidKeyLength,
    #[error("Unexpected parameter, key: {0:?}, value: {1:?}")]
    UnexpectedParameter(String, String),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Params {
    pub relay_protocol: String,
    pub sym_key: StaticSecret,
    pub relay_data: Option<String>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            relay_protocol: RELAY_PROTOCOL.to_string(),
            sym_key: StaticSecret::random_from_rng(rand::thread_rng()),
            relay_data: None,
        }
    }
}

/// https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1328.md
#[derive(Clone, Serialize, Deserialize)]
pub struct Pairing {
    pub topic: Topic,
    pub version: String,
    pub params: Params,
}

impl Default for Pairing {
    fn default() -> Self {
        Self {
            topic: Topic::generate(),
            version: "2".to_string(),
            params: Default::default(),
        }
    }
}

impl Pairing {
    fn parse_topic_and_version(path: &str) -> Result<(Topic, String), ParseError> {
        let caps = Regex::new(r"^(?P<topic>[[:word:]-]+)@(?P<version>\d+)$")
            .expect("invalid regex")
            .captures(path)
            .ok_or(ParseError::InvalidTopicAndVersion)?;
        let topic = caps
            .name("topic")
            .ok_or(ParseError::TopicNotFound)?
            .as_str()
            .to_owned();
        let topic = Topic::from(topic.parse::<DecodedTopic>().unwrap());
        let version = caps
            .name("version")
            .ok_or(ParseError::VersionNotFound)?
            .as_str()
            .to_owned();
        Ok((topic, version))
    }

    fn parse_params(url: &Url) -> Result<Params, ParseError> {
        let queries = url.query_pairs();

        let mut relay_protocol: Option<String> = None;
        let mut sym_key: Option<String> = None;
        let mut relay_data: Option<String> = None;
        for (k, v) in queries {
            match k.as_ref() {
                "relay-protocol" => relay_protocol = Some((*v).to_owned()),
                "symKey" => sym_key = Some((*v).to_owned()),
                "relay-data" => relay_data = Some((*v).to_owned()),
                _ => {
                    return Err(ParseError::UnexpectedParameter(
                        (*k).to_owned(),
                        (*v).to_owned(),
                    ))
                }
            }
        }
        let s = data_encoding::HEXLOWER_PERMISSIVE
            .decode(sym_key.ok_or(ParseError::KeyNotFound)?.as_bytes())?;
        let s: [u8; 32] = s.try_into().map_err(|_| ParseError::InvalidKeyLength)?;
        Ok(Params {
            relay_protocol: relay_protocol.ok_or(ParseError::RelayProtocolNotFound)?,
            sym_key: StaticSecret::from(s),
            relay_data,
        })
    }
}

impl Debug for Pairing {
    /// Debug with key masked.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WalletConnectUrl")
            .field("topic", &self.topic)
            .field("version", &self.version)
            .field("relay-protocol", &self.params.relay_protocol)
            .field("key", &"***")
            .field(
                "relay-data",
                &self.params.relay_data.as_deref().unwrap_or(""),
            )
            .finish()
    }
}

impl Display for Pairing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "wc:{}@2?relay-protocol={}&symKey={}",
            self.topic,
            self.params.relay_protocol,
            DecodedSymKey::from_key(&self.params.sym_key.to_bytes())
        )
    }
}

impl PartialEq for Pairing {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl Eq for Pairing {}

impl FromStr for Pairing {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = Url::from_str(s)?;

        if url.scheme() != "wc" {
            return Err(ParseError::UnexpectedProtocol {
                protocol: url.scheme().to_owned(),
            });
        }

        let (topic, version) = Self::parse_topic_and_version(url.path())?;
        Ok(Self {
            topic,
            version,
            params: Self::parse_params(&url)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;

    use super::*;

    #[test]
    fn parse_uri() {
        let uri = "wc:c9e6d30fb34afe70a15c14e9337ba8e4d5a35dd695c39b94884b0ee60c69d168@2?relay-protocol=waku&symKey=7ff3e362f825ab868e20e767fe580d0311181632707e7c878cbeca0238d45b8b";

        let actual = Pairing {
            topic: Topic::from("c9e6d30fb34afe70a15c14e9337ba8e4d5a35dd695c39b94884b0ee60c69d168")
                .to_owned(),
            version: "2".to_owned(),
            params: Params {
                relay_protocol: "waku".to_owned(),
                sym_key: hex!("7ff3e362f825ab868e20e767fe580d0311181632707e7c878cbeca0238d45b8b")
                    .into(),
                relay_data: None,
            },
        };
        let expected = Pairing::from_str(uri).unwrap();

        assert_eq!(actual, expected);
    }
}
