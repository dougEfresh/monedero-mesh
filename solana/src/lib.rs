mod compute_budget;
mod error;
mod fee;
mod memo;
mod signer;
mod stake;
mod token;
mod wallet;

use {
    base64::{prelude::BASE64_STANDARD, Engine},
    monedero_mesh::{
        rpc::{RequestMethod, RequestParams, SessionRequestRequest},
        ClientSession,
    },
    monedero_domain::namespaces::{ChainType, NamespaceName, SolanaMethod},
    serde::{Deserialize, Serialize},
    solana_program::native_token::LAMPORTS_PER_SOL,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{
        message::Message,
        pubkey::Pubkey,
        signature::Signature,
        transaction::Transaction,
    },
    spl_token_client::client::RpcClientResponse,
    std::{
        fmt::{Debug, Display, Formatter},
        ops::Deref,
        str::FromStr,
        sync::Arc,
    },
};
pub use {
    error::Error,
    memo::*,
    monedero_domain::namespaces::ChainId,
    signer::ReownSigner,
    stake::*,
    token::*,
    wallet::*,
};

pub type Result<T> = std::result::Result<T, Error>;
pub use monedero_mesh;
pub(crate) const DEFAULT_MEMO: &str = "🛠️ by github.com/dougEfresh/monedero-mesh";

async fn finish_tx(client: Arc<RpcClient>, rpc_response: &RpcClientResponse) -> Result<Signature> {
    match rpc_response {
        RpcClientResponse::Signature(s) => match client.confirm_transaction(s).await? {
            true => Ok(s.clone()),
            false => Err(Error::ConfirmationFailure(s.clone())),
        },
        RpcClientResponse::Transaction(_) => unreachable!(),
        RpcClientResponse::Simulation(_) => unreachable!(),
    }
}

pub enum Network {
    Mainnet,
    Devnet,
}

pub(crate) fn bytes_to_str(b: &[u8]) -> String {
    String::from(std::str::from_utf8(b).expect("this should not fail"))
}

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
        let decoded = solana_sdk::bs58::decode(&value.signature)
            .into_vec()
            .map_err(|e| crate::Error::Bs58Error(e.to_string()))?;
        Signature::try_from(decoded).map_err(|_| crate::Error::InvalidSignature(value))
    }
}

#[derive(Clone)]
pub struct SolanaSession {
    pk: Pubkey,
    chain: ChainId,
    session: ClientSession,
}

fn fmt_common(s: &SolanaSession) -> String {
    let c = match s.chain {
        ChainId::Solana(ChainType::Main) => "main".to_string(),
        ChainId::Solana(ChainType::Dev) => "dev".to_string(),
        ChainId::Solana(ChainType::Test) => "test".to_string(),
        _ => "unknown".to_string(),
    };
    format!("pk={} chain={c}", s.pk)
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
        Ok(Self {
            pk: Pubkey::from_str(&account.address)?,
            chain: account.chain.clone(),
            session: value.clone(),
        })
    }
}

pub(crate) fn serialize_raw_message(message: Vec<u8>) -> Result<String> {
    let msg: Message = bincode::deserialize(&message)?;
    serialize_message(msg)
}

pub(crate) fn serialize_message(message: Message) -> Result<String> {
    let transaction = Transaction::new_unsigned(message);
    let hash = bincode::serialize(&transaction)?;
    Ok(BASE64_STANDARD.encode(hash))
}

impl SolanaSession {
    pub fn pubkey(&self) -> Pubkey {
        self.pk
    }

    pub fn chain(&self) -> ChainId {
        self.chain.clone()
    }

    pub async fn balance(&self, rpc: &RpcClient) -> f64 {
        let balance = rpc.get_balance(&self.pk).await.ok().unwrap_or_default();
        (balance as f64) / (LAMPORTS_PER_SOL as f64)
    }

    pub fn chain_type(&self) -> String {
        match self.chain {
            ChainId::Solana(ChainType::Main) => "main".to_string(),
            ChainId::Solana(ChainType::Test) => "dev".to_string(),
            _ => "unknown".to_string(),
        }
    }

    pub async fn send_wallet_connect(
        &self,
        tx: WalletConnectTransaction,
    ) -> Result<SolanaSignatureResponse> {
        let params: RequestParams = RequestParams::SessionRequest(SessionRequestRequest {
            request: RequestMethod {
                method: monedero_domain::namespaces::Method::Solana(SolanaMethod::SignTransaction),
                params: serde_json::to_value(tx)?,
                expiry: None,
            },
            chain_id: self.chain.clone().into(),
        });
        self.session
            .publish_request(params)
            .await
            .map_err(|e| e.into())
    }
}
