mod error;
mod signer;
mod stake;
mod token;

use std::fmt::{Debug, Display, Formatter};
pub use token::*;

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
pub use error::Error;
use monedero_mesh::rpc::{RequestMethod, RequestParams, SessionRequestRequest};
use monedero_mesh::ClientSession;
use monedero_namespaces::{ChainId, ChainType, NamespaceName, SolanaMethod};
use serde::{Deserialize, Serialize};
pub use signer::WalletConnectSigner;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::Transaction;
use spl_token_client::client::RpcClientResponse;
pub use stake::*;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
pub type Result<T> = std::result::Result<T, Error>;
pub use monedero_mesh;

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
        ChainId::Solana(ChainType::Test) => "dev".to_string(),
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

    pub async fn send_wallet_connect(
        &self,
        tx: WalletConnectTransaction,
    ) -> Result<SolanaSignatureResponse> {
        let params: RequestParams = RequestParams::SessionRequest(SessionRequestRequest {
            request: RequestMethod {
                method: monedero_namespaces::Method::Solana(SolanaMethod::SignTransaction),
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
