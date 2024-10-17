use crate::token::TokenMetadata;
use crate::{ReownSigner, Result, SolanaSession, TokenAccounts};
use chrono::{DateTime, Utc};
use monedero_mesh::KvStorage;
use solana_account_decoder::parse_token::{UiTokenAccount, UiTokenAmount};
use solana_program::pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::request::TokenAccountsFilter;
use solana_rpc_client_api::response::RpcKeyedAccount;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use spl_token_2022::extension::StateWithExtensionsOwned;
use spl_token_2022::state::Mint;
use spl_token_client::client::RpcClientResponse;
use spl_token_client::{
    client::{ProgramClient, ProgramRpcClient, ProgramRpcClientSendTransaction},
    token::{ComputeUnitLimit, Token},
};
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct TokenMetadataClient {
    storage: KvStorage,
    client: reqwest::Client,
}

const TOKEN_METADATA_KEY: &str = "token_last_updated_at";
const JUP_TOKEN_API: &str = "https://tokens.jup.ag/tokens?tags=verified";
// Token: DEV USDC (USDC)
const USDC_ADDRESS_DEV: Pubkey = Pubkey::new_from_array([
    0x3b, 0x44, 0x2c, 0xb3, 0x91, 0x21, 0x57, 0xf1, 0x3a, 0x93, 0x3d, 0x01, 0x34, 0x28, 0x2d, 0x03,
    0x2b, 0x5f, 0xfe, 0xcd, 0x01, 0xa2, 0xdb, 0xf1, 0xb7, 0x79, 0x06, 0x08, 0xdf, 0x00, 0x2e, 0xa7,
]);

impl TokenMetadataClient {
    pub async fn init(storage_path: PathBuf) -> Result<Self> {
        let storage = KvStorage::path(storage_path, "tokens")?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(7))
            .connect_timeout(Duration::from_secs(2))
            .build()?;
        let me = Self { storage, client };
        me.populate_db().await?;
        Ok(me)
    }

    async fn populate_db(&self) -> Result<()> {
        let last_update: DateTime<Utc> = self.storage.get(TOKEN_METADATA_KEY)?.unwrap_or_default();
        let now = Utc::now();
        let since = now.signed_duration_since(last_update);
        if since.num_days() >= 1 {
            tracing::info!(
                "getting token metadata from {} ({})",
                JUP_TOKEN_API,
                since.num_days()
            );
            let response = self
                .client
                .get(String::from(JUP_TOKEN_API))
                .send()
                .await?
                .text()
                .await?;
            let tokens: Vec<TokenMetadata> = serde_json::from_str(&response)?;
            self.storage.set("tokens", tokens.clone())?;
            for t in tokens {
                self.storage.set(t.address.to_string(), t)?;
            }
            self.storage.set(
                USDC_ADDRESS_DEV.to_string(),
                TokenMetadata {
                    address: USDC_ADDRESS_DEV,
                    name: "USDC devnet".to_string(),
                    symbol: "USDC".to_string(),
                    decimals: 6,
                    mint_authority: None,
                },
            )?;
            self.storage.set(TOKEN_METADATA_KEY, now)?;
        }
        Ok(())
    }

    pub fn get(&self, token: &UiTokenAccount) -> TokenMetadata {
        self.storage
            .get::<TokenMetadata>(&token.mint.to_string())
            .ok()
            .flatten()
            .unwrap_or_else(|| default_token_metadata(token))
    }
}

/// Note, the Pubkey::from_str should not fail, as this data came from the blockchain
fn default_token_metadata(token: &UiTokenAccount) -> TokenMetadata {
    TokenMetadata {
        address: Pubkey::from_str(&token.mint).expect("this should not happen!"),
        name: token.mint.clone(),
        symbol: "UNKNOWN".to_string(),
        decimals: token.token_amount.decimals,
        mint_authority: None,
    }
}

pub struct TokenAccountsClient {
    client: Arc<RpcClient>,
    pub(super) owner: Pubkey,
    pub(super) metadata_client: TokenMetadataClient,
}

impl TokenAccountsClient {
    pub fn new(
        owner: Pubkey,
        client: Arc<RpcClient>,
        metadata_client: TokenMetadataClient,
    ) -> Self {
        Self {
            client,
            owner,
            metadata_client,
        }
    }

    pub async fn accounts(&self) -> Result<TokenAccounts> {
        let filters = vec![
            TokenAccountsFilter::ProgramId(spl_token::id()),
            TokenAccountsFilter::ProgramId(spl_token_2022::id()),
        ];
        let mut accounts = vec![];
        for filter in filters {
            accounts.push(
                self.client
                    .get_token_accounts_by_owner(&self.owner, filter)
                    .await?,
            );
        }
        let accounts: Vec<RpcKeyedAccount> = accounts.into_iter().flatten().collect();
        self.sort_and_parse_token_accounts(accounts, false)
    }
}

#[derive(Clone)]
pub struct TokenTransferClient {
    account: Pubkey,
    signer: ReownSigner,
    token: Arc<Token<ProgramRpcClientSendTransaction>>,
    client: Arc<RpcClient>,
    program_id: Pubkey,
}

impl PartialEq for TokenTransferClient {
    fn eq(&self, other: &Self) -> bool {
        self.account.eq(&other.account) && self.program_id.eq(&other.program_id)
    }
}

impl Debug for TokenTransferClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[token-client] {} program: {} ",
            self.account, self.program_id
        )
    }
}

impl TokenTransferClient {
    pub async fn init(
        signer: ReownSigner,
        client: Arc<RpcClient>,
        token_address: impl Into<Pubkey>,
        program_id: Pubkey,
    ) -> Result<Self> {
        let token_address = token_address.into();
        let tc: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
            ProgramRpcClient::new(client.clone(), ProgramRpcClientSendTransaction),
        );
        let token = Token::new(
            tc,
            &program_id,
            &token_address,
            None,
            Arc::new(signer.clone()),
        );
        let account = token.get_associated_token_address(&signer.pubkey());
        Ok(Self {
            signer,
            account,
            token: Arc::new(token),
            client,
            program_id,
        })
    }

    pub fn init_wrap_native(
        signer: ReownSigner,
        client: Arc<RpcClient>,
        program_id: Pubkey,
    ) -> Self {
        let tc: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
            ProgramRpcClient::new(client.clone(), ProgramRpcClientSendTransaction),
        );
        let token = Token::new_native(tc, &program_id, Arc::new(signer.clone()));
        let account = token.get_associated_token_address(&signer.pubkey());
        Self {
            signer,
            account,
            token: Arc::new(token),
            client,
            program_id,
        }
    }

    pub fn account(&self) -> &Pubkey {
        &self.account
    }

    pub async fn balance(&self) -> Result<u64> {
        let info = self.token.get_account_info(&self.account).await?;

        Ok(info.base.amount)
    }

    pub async fn transfer(&self, to: &Pubkey, amt: u64) -> Result<Signature> {
        let to_account = self.token.get_associated_token_address(to);
        tracing::info!("destination account {to_account}");
        let result = self
            .token
            .create_recipient_associated_account_and_transfer(
                &self.account,
                &to_account,
                &to,
                &self.signer.pubkey(),
                amt,
                None,
                &[&self.signer],
            )
            .await?;
        crate::finish_tx(self.client.clone(), &result).await
    }

    pub async fn mint_to(&self, to: &Pubkey, amount: u64) -> Result<Signature> {
        // TODO optimize to one transaction
        self.token.get_or_create_associated_account_info(to).await?;
        let to_account = self.token.get_associated_token_address(to);
        let result = self
            .token
            .mint_to(&to_account, &self.signer.pubkey(), amount, &[&self.signer])
            .await?;
        crate::finish_tx(self.client.clone(), &result).await
    }

    pub async fn wrap(&self, amount: u64, immutable_owner: bool) -> Result<Signature> {
        if immutable_owner && self.program_id == spl_token::id() {
            return Err(crate::Error::InvalidTokenProgram);
        }
        if immutable_owner {
            let result = self
                .token
                .wrap(
                    &self.account,
                    &self.signer.pubkey(),
                    amount,
                    &[&self.signer],
                )
                .await?;
            return crate::finish_tx(self.client.clone(), &result).await;
        }
        let result = self
            .token
            .wrap_with_mutable_ownership(
                &self.account,
                &self.signer.pubkey(),
                amount,
                &[&self.signer],
            )
            .await?;
        crate::finish_tx(self.client.clone(), &result).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_token_metadta() -> anyhow::Result<()> {
        let storage_path = PathBuf::from_str(&format!("{}/tokens-test", env!("CARGO_TARGET_DIR")))?;
        let kv = KvStorage::path(storage_path.clone(), "tokens")?;
        Ok(())
    }
}
