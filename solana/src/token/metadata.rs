use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use monedero_mesh::KvStorage;
use serde::{Deserialize, Serialize};
use solana_account_decoder::parse_token::UiTokenAccount;
use solana_program::pubkey::Pubkey;

use crate::TokenMetadata;

const TOKEN_METADATA_KEY: &str = "token_last_updated_at";
const JUP_TOKEN_API: &str = "https://tokens.jup.ag/tokens?tags=verified";
// Token: DEV USDC (USDC)
const USDC_ADDRESS_DEV: Pubkey = Pubkey::new_from_array([
    0x3b, 0x44, 0x2c, 0xb3, 0x91, 0x21, 0x57, 0xf1, 0x3a, 0x93, 0x3d, 0x01, 0x34, 0x28, 0x2d, 0x03,
    0x2b, 0x5f, 0xfe, 0xcd, 0x01, 0xa2, 0xdb, 0xf1, 0xb7, 0x79, 0x06, 0x08, 0xdf, 0x00, 0x2e, 0xa7,
]);

#[derive(Clone)]
pub struct TokenMetadataClient {
    storage: KvStorage,
    client: reqwest::Client,
    tokens: Arc<DashMap<Pubkey, TokenMetadata>>,
}

impl Debug for TokenMetadataClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TokenMetadataClient")
    }
}

impl TokenMetadataClient {
    #[tracing::instrument(level = "debug")]
    pub async fn init(storage_path: PathBuf) -> crate::Result<Self> {
        let storage = KvStorage::path(storage_path, "tokens")?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(7))
            .connect_timeout(Duration::from_secs(2))
            .build()?;
        let mut me = Self {
            storage,
            client,
            tokens: Arc::new(DashMap::new()),
        };
        me.populate_db().await?;
        Ok(me)
    }

    async fn populate_db(&mut self) -> crate::Result<()> {
        let last_update: DateTime<Utc> = self.storage.get(TOKEN_METADATA_KEY)?.unwrap_or_default();
        let now = Utc::now();
        let since = now.signed_duration_since(last_update);
        let mut tokens: Vec<TokenMetadata> = self.storage.get("token")?.unwrap_or_default();
        if since.num_days() >= 1 || tokens.is_empty() {
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
            tokens = serde_json::from_str::<Vec<TokenMetadata>>(&response)?;

            tokens.push(TokenMetadata {
                address: USDC_ADDRESS_DEV,
                name: "USDC devnet".to_string(),
                symbol: "USDC".to_string(),
                decimals: 6,
                mint_authority: None,
            });
            self.storage.set("tokens", tokens.clone())?;
            self.storage.set(TOKEN_METADATA_KEY, now)?;
        }
        let tokens = DashMap::from_iter(tokens.into_iter().map(|t| (t.address, t)));
        self.tokens = Arc::new(tokens);
        Ok(())
    }

    pub fn get(&self, token: &UiTokenAccount) -> TokenMetadata {
        let pk: Pubkey = Pubkey::from_str(&token.mint).unwrap();
        self.tokens
            .entry(pk)
            .or_insert_with(|| default_token_metadata(token))
            .value()
            .clone()
    }
}

/// Note, the Pubkey::from_str should not fail, as this data came from the blockchain
fn default_token_metadata(token: &UiTokenAccount) -> TokenMetadata {
    TokenMetadata {
        address: Pubkey::from_str(&token.mint).expect("this should not happen!"),
        name: token.mint.clone(),
        symbol: String::from(&token.mint.as_str()[0..6]),
        decimals: token.token_amount.decimals,
        mint_authority: None,
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn blah() {}

    #[tokio::test]
    async fn token_metadata() -> anyhow::Result<()> {
        let storage_path = PathBuf::from_str(&format!("{}/tokens-test", env!("CARGO_TARGET_DIR")))?;
        let kv = KvStorage::path(storage_path.clone(), "tokens")?;
        let md = TokenMetadataClient::init(storage_path).await?;
        Ok(())
    }
}
