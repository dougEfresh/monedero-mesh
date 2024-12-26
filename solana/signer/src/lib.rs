mod error;
mod signer;

#[cfg(not(target_family = "wasm"))]
pub use monedero_mesh::MockRelay;
pub use {
    error::Error,
    monedero_mesh::{
        self as session,
        domain::{self, ProjectId},
        spawn_task,
        Dapp,
        KvStorage,
        KvStorageError,
        Metadata,
        ReownBuilder,
    },
    signer::ReownSigner,
};
use {
    monedero_mesh::{
        domain::namespaces::{ChainId, ChainType, Method, NamespaceName, SolanaMethod},
        rpc::{RequestMethod, RequestParams, SessionRequestRequest},
        ClientSession,
    },
    solana_pubkey::Pubkey,
    solana_signature::Signature,
    std::{
        fmt::{Debug, Display, Formatter},
        ops::Deref,
        str::FromStr,
    },
};

mod rpc;
pub use rpc::*;
mod mock;
pub use mock::*;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
pub struct SolanaSession {
    pk: Pubkey,
    chain: ChainId,
    session: ClientSession,
    network: ChainType,
}

fn fmt_common(s: &SolanaSession) -> String {
    format!("pk={},chain={}", s.pk, s.chain)
}

impl Eq for SolanaSession {}

impl PartialEq for SolanaSession {
    fn eq(&self, other: &Self) -> bool {
        self.pk.eq(&other.pk)
    }
}

impl Debug for SolanaSession {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", fmt_common(self))
    }
}

impl Display for SolanaSession {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", fmt_common(self))
    }
}

impl TryFrom<&ClientSession> for SolanaSession {
    type Error = Error;

    fn try_from(value: &ClientSession) -> std::result::Result<Self, Self::Error> {
        let ns = value
            .namespaces()
            .get(&NamespaceName::Solana)
            .ok_or(Error::NoSolanaNamespace)?;
        let account = ns
            .accounts
            .deref()
            .first()
            .ok_or(Error::SolanaAccountNotFound)?;
        let network = match &account.chain {
            ChainId::Solana(ChainType::Test) => ChainType::Test,
            ChainId::Solana(ChainType::Main) => ChainType::Main,
            _ => ChainType::Dev,
        };
        Ok(Self {
            pk: Pubkey::from_str(&account.address)?,
            chain: account.chain.clone(),
            session: value.clone(),
            network,
        })
    }
}

impl SolanaSession {
    pub fn pubkey(&self) -> Pubkey {
        self.pk
    }

    pub fn chain(&self) -> ChainId {
        self.chain.clone()
    }

    pub fn network(&self) -> ChainType {
        self.network
    }

    pub async fn sign_message_payload(&self, payload: SignMessageRequest) -> Result<Signature> {
        let params: RequestParams = RequestParams::SessionRequest(SessionRequestRequest {
            request: RequestMethod {
                method: Method::Solana(SolanaMethod::SignMessage),
                params: serde_json::to_value(&payload)?,
                expiry: None,
            },
            chain_id: self.chain.clone(),
        });
        let response: SolanaSignatureResponse = self.session.publish_request(params).await?;
        Signature::try_from(response)
    }

    pub async fn sign_message(&self, message: &str) -> Result<Signature> {
        let m = SignMessageRequest::new(self.pubkey(), message);
        self.sign_message_payload(m).await
    }

    pub async fn sign_transaction(&self, tx: WalletConnectTransaction) -> Result<Signature> {
        let params: RequestParams = RequestParams::SessionRequest(SessionRequestRequest {
            request: RequestMethod {
                method: Method::Solana(SolanaMethod::SignTransaction),
                params: serde_json::to_value(tx)?,
                expiry: None,
            },
            chain_id: self.chain.clone(),
        });
        let response: SolanaSignatureResponse = self.session.publish_request(params).await?;
        Signature::try_from(response)
    }
}
