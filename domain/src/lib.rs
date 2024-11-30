use {
    derive_more::{AsMut, AsRef},
    monedero_relay::ed25519_dalek::SecretKey,
    serde::{Deserialize, Serialize},
};
pub use {
    monedero_namespaces as namespaces,
    monedero_relay::{
        auth_token,
        DecodedTopic,
        Message,
        MessageId,
        MessageIdGenerator,
        PairingTopic,
        ProjectId,
        SessionTopic,
        SubscriptionId,
        Topic,
    },
};

pub mod pairing_uri;
pub use pairing_uri::Pairing;

const MULTICODEC_ED25519_LENGTH: usize = 32;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, AsRef, AsMut, Serialize, Deserialize)]
#[as_ref(forward)]
#[as_mut(forward)]
pub struct DecodedSymKey(pub [u8; MULTICODEC_ED25519_LENGTH]);

impl DecodedSymKey {
    #[inline]
    pub fn from_key(key: &SecretKey) -> Self {
        Self(*key)
    }
}

impl std::fmt::Display for DecodedSymKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&data_encoding::HEXLOWER_PERMISSIVE.encode(&self.0))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionSettled {
    pub topic: SessionTopic,
    pub namespaces: monedero_namespaces::Namespaces,
    /// Unix timestamp.
    ///
    /// Expiry should be between .now() + TTL.
    pub expiry: i64,
}
