mod error;
mod signer;

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
    serde::{Deserialize, Serialize},
    solana_pubkey::Pubkey,
    solana_signature::Signature,
    std::{
        fmt::{Debug, Display, Formatter},
        ops::Deref,
        str::FromStr,
    },
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletConnectTransaction {
    pub transaction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaSignatureResponse {
    pub signature: String,
}

impl TryFrom<SolanaSignatureResponse> for Signature {
    type Error = Error;

    fn try_from(value: SolanaSignatureResponse) -> std::result::Result<Self, Self::Error> {
        let decoded: Vec<u8> = bs58::decode(&value.signature)
            .into_vec()
            .map_err(|e| Error::Bs58Error(e.to_string()))?;
        let array: [u8; 64] = decoded
            .try_into()
            .map_err(|_| Error::SigError(value.signature))?;
        Ok(Signature::from(array))
    }
}

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
            ChainId::Solana(ChainType::Dev) => ChainType::Dev,
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
        self.network.clone()
    }

    pub async fn sign_transaction(&self, tx: WalletConnectTransaction) -> Result<Signature> {
        let params: RequestParams = RequestParams::SessionRequest(SessionRequestRequest {
            request: RequestMethod {
                method: Method::Solana(SolanaMethod::SignTransaction),
                params: serde_json::to_value(tx)?,
                expiry: None,
            },
            chain_id: self.chain.clone().into(),
        });
        let response: SolanaSignatureResponse = self.session.publish_request(params).await?;
        Signature::try_from(response)
    }
}
