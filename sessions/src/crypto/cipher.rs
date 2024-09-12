use crate::crypto::error::CipherError;
use crate::pairing_uri::Pairing;
use crate::rpc::SessionSettleRequest;
use crate::{KvStorage, SessionSettled, SessionTopic};
use chacha20poly1305::{aead::Aead, AeadCore, ChaCha20Poly1305, KeyInit, Nonce};
use dashmap::DashMap;
use derive_more::{AsMut, AsRef};
use hkdf::Hkdf;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use tracing::debug;
use monedero_relay::ed25519_dalek::{SecretKey, VerifyingKey};
use monedero_relay::{DecodedTopic, Topic};
use x25519_dalek::{PublicKey, StaticSecret};

pub const MULTICODEC_ED25519_LENGTH: usize = 32;
const CRYPTO_STORAGE_PREFIX_KEY: &str = "crypto";

pub type AtomicPairing = Arc<DashMap<Topic, Arc<Pairing>>>;
type CipherSessionKeyStore = Arc<DashMap<Topic, ChaCha20Poly1305>>;

#[derive(Debug, Clone, Copy, Default)]
pub enum Type {
    #[default]
    Type0,
    Type1(VerifyingKey),
}

impl Type {
    fn as_bytes(&self) -> Vec<u8> {
        match self {
            Type::Type1(key) => {
                let mut envelope = vec![1u8];
                envelope.extend(key.as_bytes().to_vec());
                envelope
            }
            _ => vec![0u8],
        }
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes[0] {
            0u8 => Some(Self::Type0),
            1u8 => match VerifyingKey::from_bytes((&bytes[1..32]).try_into().unwrap()) {
                Ok(key) => Some(Self::Type1(key)),
                _ => None,
            },
            _ => None,
        }
    }
}

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

#[derive(Clone, xtra::Actor)]
pub struct Cipher {
    ciphers: CipherSessionKeyStore,
    pairing: AtomicPairing,
    storage: Arc<KvStorage>,
}

impl Debug for Cipher {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ciphers={} pairings={}",
            self.ciphers.len(),
            self.pairing.len()
        )
    }
}

impl Cipher {
    fn storage_pairing() -> String {
        format!("{CRYPTO_STORAGE_PREFIX_KEY}-pairingtopic")
    }

    fn storage_sessions() -> String {
        format!("{CRYPTO_STORAGE_PREFIX_KEY}-sessions")
    }

    fn storage_session_key(topic: &Topic) -> String {
        format!("{CRYPTO_STORAGE_PREFIX_KEY}-{topic}")
    }

    fn storage_settlement(topic: &Topic) -> String {
        format!("{CRYPTO_STORAGE_PREFIX_KEY}-settlement-{}", topic)
    }
}

impl Cipher {
    /// Create a new Cipher keystore base on pairing_topic or generate a new one
    /// https://specs.walletconnect.com/2.0/specs/clients/core/pairing/pairing-uri
    pub fn new(
        storage: Arc<KvStorage>,
        _pairing_topic: Option<Topic>,
    ) -> Result<Self, CipherError> {
        let storage_pairing_key = Self::storage_pairing();
        let pairings = DashMap::new();
        if let Some(pairing) = storage.get::<Pairing>(storage_pairing_key)? {
            pairings.insert(pairing.topic.clone(), Arc::new(pairing));
        }
        let cipher = Self {
            ciphers: Arc::new(DashMap::new()),
            pairing: Arc::new(pairings),
            storage,
        };
        cipher.init()?;
        Ok(cipher)
    }

    fn init(&self) -> Result<(), CipherError> {
        let mut session_expired = false;
        match self.pairing() {
            Some(pairing) => {
                debug!("found existing pairing...restoring");
                self.pairing
                    .insert(pairing.topic.clone(), Arc::new(pairing.clone()));
                let key = pairing.params.sym_key.clone();
                self.ciphers.insert(
                    pairing.topic.clone(),
                    ChaCha20Poly1305::new((&key.to_bytes()).into()),
                );
                let sessions_key = format!("{CRYPTO_STORAGE_PREFIX_KEY}-sessions");
                if let Some(sessions) = self.storage.get::<Vec<String>>(&sessions_key)? {
                    debug!("restoring {} sessions", sessions.len());
                    for s in sessions {
                        // TODO: Do I need to copy the string?
                        if self
                            .is_expired(Topic::from(String::from(&s)))
                            .ok()
                            .unwrap_or(false)
                        {
                            session_expired = true;
                            break;
                        }
                        if let Some(controller_pk) = self
                            .storage
                            .get::<String>(Self::storage_session_key(&Topic::from(s)))?
                        {
                            let (topic, expanded_key) =
                                Self::derive_sym_key(key.clone(), controller_pk)?;
                            let _ = self.register(topic, expanded_key);
                        }
                    }
                    if session_expired {
                        tracing::info!("Session has expired, resetting storage");
                        self.storage.clear();
                    }
                }
            }
            None => {
                debug!("clearing session storage");
                self.storage.clear();
            }
        };
        Ok(())
    }

    pub(crate) fn set_settlement(&self, settlement: SessionSettled) -> Result<(), CipherError> {
        let sessions_key = Self::storage_settlement(&settlement.topic);
        self.storage.set(sessions_key, settlement)?;
        Ok(())
    }

    pub(crate) fn settlements(&self) -> Result<Vec<SessionSettled>, CipherError> {
        if self.pairing.is_empty() || self.ciphers.is_empty() {
            return Ok(Vec::new());
        }

        let sessions: Vec<Topic> = self
            .storage
            .get(&Self::storage_sessions())?
            .unwrap_or_default();
        let mut settled: Vec<SessionSettled> = Vec::new();
        for topic in sessions {
            if let Some(s) = self
                .storage
                .get::<SessionSettled>(Self::storage_settlement(&topic))?
            {
                settled.push(s);
            }
        }
        Ok(settled)
    }

    pub(crate) fn is_expired(&self, topic: Topic) -> Result<bool, CipherError> {
        let sessions_key = format!("{CRYPTO_STORAGE_PREFIX_KEY}-settlement-{}", topic);
        let session: SessionSettleRequest = self
            .storage
            .get(sessions_key)?
            .ok_or(CipherError::UnknownSessionTopic(topic))?;
        let now = chrono::Utc::now().timestamp();
        Ok(session.expiry < now)
    }

    #[tracing::instrument(level = "info", fields(topic = crate::shorten_topic(topic)))]
    pub(crate) fn delete_session(&self, topic: &Topic) -> Result<(), CipherError> {
        self.storage.delete(Self::storage_session_key(topic))?;
        if let Some(sessions) = self.storage.get::<Vec<Topic>>(Self::storage_sessions())? {
            let new_sessions: Vec<Topic> = sessions.into_iter().filter(|t| t == topic).collect();
            self.storage.set(Self::storage_sessions(), new_sessions)?;
        }
        let sessions_key = Self::storage_settlement(&topic);
        self.storage.delete(sessions_key)?;
        self.ciphers.remove(topic);
        Ok(())
    }

    pub fn set_pairing(&self, pairing: Option<Pairing>) -> Result<(), CipherError> {
        self.reset();
        if let Some(new_pair) = pairing {
            debug!("setting pairing topic to {}", new_pair.topic);
            self.storage
                .set::<Pairing>(Self::storage_pairing(), new_pair.clone())?;
            self.pairing
                .insert(new_pair.topic.clone(), Arc::new(new_pair.clone()));
            let key = new_pair.params.sym_key.clone();
            self.ciphers.insert(
                new_pair.topic,
                ChaCha20Poly1305::new((&key.to_bytes()).into()),
            );
        }
        Ok(())
    }

    pub fn public_key(&self) -> Option<PublicKey> {
        if let Some(pairing) = self.pairing() {
            return Some(PublicKey::from(&pairing.params.sym_key));
        }
        None
    }

    pub fn public_key_hex(&self) -> Option<String> {
        if let Some(ref pk) = self.public_key() {
            return Some(data_encoding::HEXLOWER_PERMISSIVE.encode(pk.as_bytes()));
        }
        None
    }

    pub fn pairing_uri(&self) -> Option<String> {
        self.pairing().map(|p| p.to_string())
    }

    pub fn pairing_key(&self) -> Option<StaticSecret> {
        if let Some(pairing) = self.pairing() {
            return Some(pairing.params.sym_key.clone());
        }
        None
    }

    pub fn pairing(&self) -> Option<Pairing> {
        self.storage
            .get(Self::storage_pairing())
            .ok()
            .unwrap_or_else(|| None)
    }

    pub fn create_common_topic(
        &self,
        controller_pk: String,
    ) -> Result<(Topic, PublicKey), CipherError> {
        let pairing_key = self.pairing_key().ok_or(CipherError::NonExistingPairing)?;
        let (new_topic, expanded_key) =
            Self::derive_sym_key(pairing_key, String::from(&controller_pk))?;
        self.update_sessions(controller_pk, new_topic.clone())?;
        let _ = self.register(new_topic.clone(), expanded_key.clone());
        Ok((new_topic, PublicKey::from(&expanded_key)))
    }

    fn update_sessions(&self, controller_pk: String, topic: Topic) -> Result<(), CipherError> {
        // TODO: May need to lock this entire operation
        let sessions_storage_key = Self::storage_sessions();
        let mut sessions: Vec<Topic> = self.storage.get(&sessions_storage_key)?.unwrap_or_default();
        sessions.push(topic.clone());
        self.storage.set(&sessions_storage_key, sessions)?;
        self.storage
            .set(Self::storage_session_key(&topic), controller_pk)?;
        Ok(())
    }

    pub fn derive_sym_key(
        static_key: StaticSecret,
        controller_pk: String,
    ) -> Result<(Topic, StaticSecret), CipherError> {
        //let key = DecodedClientId(
        //(&data_encoding::HEXLOWER_PERMISSIVE.decode(controller_pk.as_bytes()).unwrap())[..].try_into().unwrap(),
        //);
        let decoded = data_encoding::HEXLOWER_PERMISSIVE.decode(controller_pk.as_bytes())?;
        let k: [u8; 32] = decoded
            .try_into()
            .map_err(|_| CipherError::InvalidKeyLength)?;
        let public_key = PublicKey::from(k);
        let shared_secret = static_key.diffie_hellman(&public_key);
        let hk = Hkdf::<Sha256>::new(None, shared_secret.as_ref());
        let mut okm = [0u8; 32];
        hk.expand(&[], &mut okm).unwrap();
        let expanded_key = StaticSecret::from(okm);
        let new_topic = Topic::from(DecodedTopic(Sha256::digest(expanded_key.as_ref()).into()));
        Ok((new_topic, expanded_key))
    }

    fn register(&self, topic: Topic, key: StaticSecret) -> Result<(), CipherError> {
        self.ciphers.insert(
            topic.clone(),
            ChaCha20Poly1305::new((&key.to_bytes()).into()),
        );
        Ok(())
    }

    pub fn encode<T: Serialize>(&self, topic: &Topic, payload: &T) -> Result<String, CipherError> {
        self.encode_with_params(
            topic,
            payload,
            ChaCha20Poly1305::generate_nonce(&mut rand::thread_rng()),
            Type::default(),
        )
    }

    pub fn encode_with_params<T: Serialize>(
        &self,
        topic: &Topic,
        payload: &T,
        nonce: Nonce,
        envelope_type: Type,
    ) -> Result<String, CipherError> {
        let cipher = self
            .ciphers
            .get(topic)
            .ok_or(CipherError::UnknownTopic(topic.clone()))?;
        let serialized_payload = serde_json::to_string(payload)?;
        debug!("serialized payload for topic {topic} {serialized_payload}");
        let encrypted_payload = cipher
            .encrypt(&nonce, &*serialized_payload.into_bytes())
            .map_err(|_| CipherError::Corrupted)?;
        let mut envelope = envelope_type.as_bytes();
        envelope.extend(nonce.to_vec());
        envelope.extend(encrypted_payload.to_vec());
        Ok(data_encoding::BASE64.encode(&envelope))
    }

    pub fn decode<T: DeserializeOwned>(
        &self,
        topic: &Topic,
        payload: &str,
    ) -> Result<T, CipherError> {
        let decoded_msg = &self.decode_to_string(topic, payload)?;
        let from_str = serde_json::from_str(decoded_msg);
        Ok(from_str?)
    }

    pub(crate) fn decode_to_string(
        &self,
        topic: &Topic,
        payload: &str,
    ) -> Result<String, CipherError> {
        let encrypted_payload = data_encoding::BASE64.decode(payload.as_bytes())?;
        match Type::from_bytes(&encrypted_payload) {
            Some(Type::Type0) => self.decode_bytes(topic, &encrypted_payload[1..]),
            Some(Type::Type1(_)) => self.decode_bytes(topic, &encrypted_payload[33..]),
            _ => Err(CipherError::CorruptedPayload),
        }
    }

    fn decode_bytes(&self, topic: &Topic, bytes: &[u8]) -> Result<String, CipherError> {
        let cipher = self
            .ciphers
            .get(topic)
            .ok_or(CipherError::UnknownTopic(topic.clone()))?;
        let decoded_bytes = cipher
            .decrypt((&bytes[0..12]).into(), &bytes[12..])
            .map_err(|_| CipherError::EncryptionError)?;
        let decoded = String::from_utf8(decoded_bytes)?;
        debug!("decoded from topic {topic} {decoded}");
        Ok(decoded)
    }

    #[allow(dead_code)]
    fn session_topics(&self) -> usize {
        self.ciphers.len()
    }

    pub fn subscriptions(&self) -> Vec<Topic> {
        self.ciphers.iter().map(|k| k.key().clone()).collect()
    }

    pub fn reset(&self) {
        self.ciphers.clear();
        self.pairing.clear();
        self.storage.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::session::SessionKey;
    use crate::rpc::{
        Controller, Metadata, PairPingRequest, Request, RequestParams, SessionExtendRequest,
    };
    use crate::storage::KvStorage;
    use anyhow::format_err;
    use std::str::FromStr;
    use monedero_relay::MessageIdGenerator;

    fn temp_location() -> Option<String> {
        let topic = Topic::generate();
        Some(format!("target/kv/{topic}"))
    }

    #[test]
    pub fn test_cipher_encrypt() -> anyhow::Result<()> {
        crate::test::init_tracing();
        let dapp_store = KvStorage::file(temp_location())?;
        let wallet_store = KvStorage::file(temp_location())?;
        let dapp = Cipher::new(Arc::new(dapp_store), None)?;
        let wallet = Cipher::new(Arc::new(wallet_store), None)?;
        let pairing = Arc::new(create_pairing());
        let generator = MessageIdGenerator::new();

        dapp.set_pairing(Some((*pairing).clone()))?;
        let pairing_from_uri = Pairing::from_str(&dapp.pairing_uri().unwrap())?;
        wallet.set_pairing(Some(pairing_from_uri))?;
        assert_eq!(
            dapp.pairing().unwrap().topic,
            wallet.pairing().unwrap().topic
        );

        let (dapp_topic, _) = dapp.create_common_topic(wallet.public_key_hex().unwrap())?;
        let (session_topic, _) = wallet.create_common_topic(dapp.public_key_hex().unwrap())?;

        assert_eq!(dapp_topic, session_topic);
        //assert_eq!(wallet.public_key().unwrap(), wallet_pk);

        let dapp_req_ext = RequestParams::SessionExtend(SessionExtendRequest { expiry: 1 });
        let dapp_req_ext = Request::new(generator.next(), dapp_req_ext.clone().try_into()?);

        let encrypted = dapp.encode(&session_topic, &dapp_req_ext)?;
        let decrypted_req: Request = wallet.decode::<Request>(&session_topic, &encrypted)?;

        assert_eq!(dapp_req_ext, decrypted_req);

        let wallet_req_ext = RequestParams::SessionExtend(SessionExtendRequest { expiry: 2 });
        let wallet_req_ext = Request::new(generator.next(), wallet_req_ext.clone().try_into()?);
        let encrypted = wallet.encode(&session_topic, &wallet_req_ext)?;
        let decrypted_req: Request = dapp.decode::<Request>(&session_topic, &encrypted)?;
        assert_eq!(wallet_req_ext, decrypted_req);

        // Pairing topic peer
        let dapp_req_ext = RequestParams::PairPing(PairPingRequest {});
        let dapp_req_ext = Request::new(generator.next(), dapp_req_ext.try_into()?);
        let encrypted = dapp.encode(&pairing.topic, &dapp_req_ext)?;
        let decrypted_req = wallet.decode::<Request>(&pairing.topic, &encrypted)?;

        assert_eq!(dapp_req_ext, decrypted_req);
        Ok(())
    }

    fn create_pairing() -> Pairing {
        Default::default()
    }

    #[cfg(not(feature = "wasm"))]
    #[test]
    pub fn test_cipher_storage_os() -> anyhow::Result<()> {
        let topic = Topic::generate();
        let store = KvStorage::file(Some(format!("target/kv/{topic}")))?;
        test_storage(Arc::new(store))
    }

    #[cfg(not(feature = "wasm"))]
    #[test]
    pub fn test_cipher_storage_mem() -> anyhow::Result<()> {
        test_storage(Arc::new(KvStorage::mem()))?;
        Ok(())
    }

    fn test_storage(store: Arc<KvStorage>) -> anyhow::Result<()> {
        crate::test::init_tracing();
        let pairing = Arc::new(create_pairing());
        let pairing_key = pairing.params.sym_key.clone();
        let pairing_topic = pairing.topic.clone();
        let ciphers = Cipher::new(store.clone(), None)?;
        assert!(ciphers.pairing().is_none());
        ciphers.set_pairing(Some((*pairing).clone()))?;
        ciphers
            .pairing()
            .ok_or(format_err!("pairing should be here"))?;
        assert_eq!(ciphers.session_topics(), 1);
        assert_eq!(ciphers.pairing.len(), 1);
        drop(ciphers);

        // check pairing is restored
        let ciphers = Cipher::new(store.clone(), None)?;
        let restored_pairing = ciphers.pairing().ok_or(format_err!("pairing not here!"))?;

        assert_eq!(restored_pairing.topic, pairing_topic);
        assert_eq!(
            restored_pairing.params.sym_key.as_bytes(),
            pairing_key.as_bytes()
        );

        // Add a Session
        let session_key =
            SessionKey::from_osrng(ciphers.public_key().ok_or(format_err!("shit"))?.as_bytes())?;
        let responder_pk = session_key.public_key();
        let (session_topic, _) = ciphers.create_common_topic(String::from(&responder_pk))?;
        assert_eq!(session_topic, session_key.generate_topic());
        assert_eq!(ciphers.session_topics(), 1);

        // Validate Sessions in Store
        let kv = format!("{CRYPTO_STORAGE_PREFIX_KEY}-sessions");
        let sessions: Vec<String> = store.get::<Vec<String>>(kv)?.unwrap();
        assert_eq!(ciphers.session_topics(), sessions.len());
        let kv = format!("{CRYPTO_STORAGE_PREFIX_KEY}-{session_topic}");
        let stored_pk: String = store.get(kv)?.unwrap();
        assert_eq!(stored_pk, responder_pk);

        // Delete session
        ciphers.delete_session(&session_topic)?;
        assert_eq!(ciphers.session_topics(), 0);
        assert!(store
            .get::<Topic>(Cipher::storage_session_key(&session_topic))?
            .is_none());
        // put session back
        let _ = ciphers.create_common_topic(String::from(&responder_pk))?;
        drop(ciphers);

        // Restore sessions
        let ciphers = Cipher::new(store.clone(), None)?;
        let restored_pairing = ciphers.pairing().ok_or(format_err!("pairing not here!"))?;
        assert_eq!(ciphers.session_topics(), 1);
        assert_eq!(restored_pairing.topic, pairing_topic);

        // Settlement
        let session_key =
            SessionKey::from_osrng(ciphers.public_key().ok_or(format_err!("shit"))?.as_bytes())?;
        let responder_pk = session_key.public_key();
        let (session_topic, _) = ciphers.create_common_topic(String::from(&responder_pk))?;

        let now = chrono::Utc::now();
        let future = now + chrono::Duration::hours(24);
        let mut settlement = SessionSettleRequest {
            controller: Controller {
                public_key: ciphers
                    .public_key_hex()
                    .ok_or(CipherError::UnknownTopic(pairing_topic.clone()))?,
                metadata: Metadata::default(),
            },
            ..Default::default()
        };
        ciphers.set_settlement(session_topic.clone(), settlement.clone())?;
        assert!(!ciphers.is_expired(session_topic.clone())?);

        // get settlements
        assert_eq!(1, ciphers.settlements()?.len());

        let past = now - chrono::Duration::hours(1);
        settlement.expiry = past.timestamp();
        ciphers.set_settlement(session_topic.clone(), settlement.clone())?;
        assert!(ciphers.is_expired(session_topic.clone())?);
        drop(ciphers);
        // restore should reset / clear storage due to expired session
        let ciphers = Cipher::new(store.clone(), None)?;
        assert!(ciphers.pairing().is_none());
        assert!(ciphers.settlements()?.is_empty());

        // New Pairing
        ciphers.set_pairing(Some(create_pairing()))?;
        let kv = format!("{CRYPTO_STORAGE_PREFIX_KEY}-sessions");
        let sessions = store.get::<Vec<String>>(kv)?;
        assert!(sessions.is_none());
        let kv = format!("{CRYPTO_STORAGE_PREFIX_KEY}-{session_topic}");
        let stored_pk = store.get::<String>(kv)?;
        assert!(stored_pk.is_none());
        let kv = format!("{CRYPTO_STORAGE_PREFIX_KEY}-pairingtopic");
        let pairing = store.get::<Pairing>(kv)?;
        assert!(pairing.is_some());

        // Reset
        ciphers.reset();
        let kv = format!("{CRYPTO_STORAGE_PREFIX_KEY}-sessions");
        let sessions = store.get::<Vec<String>>(kv)?;
        assert!(sessions.is_none());
        let kv = format!("{CRYPTO_STORAGE_PREFIX_KEY}-{session_topic}");
        let stored_pk = store.get::<String>(kv)?;
        assert!(stored_pk.is_none());
        let kv = format!("{CRYPTO_STORAGE_PREFIX_KEY}-pairingtopic");
        let pairing = store.get::<Pairing>(kv)?;
        assert!(pairing.is_none());

        Ok(())
    }
}
