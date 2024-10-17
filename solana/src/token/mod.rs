mod client;
mod mint;
mod sort;
mod symbol;

use crate::Result;
pub use client::*;
pub use mint::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use solana_account_decoder::parse_token::UiTokenAccount;
use solana_program::pubkey::Pubkey;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
pub use symbol::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TokenMetadata {
    #[serde(deserialize_with = "deserialize_pk", serialize_with = "serialize_pk")]
    pub address: Pubkey,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    #[serde(
        deserialize_with = "deserialize_pk_opt",
        serialize_with = "serialize_pk_opt"
    )]
    pub mint_authority: Option<Pubkey>,
}

impl Display for TokenMetadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.symbol)
    }
}

fn deserialize_pk<'de, D>(deserializer: D) -> std::result::Result<Pubkey, D::Error>
where
    D: Deserializer<'de>,
{
    let pk = String::deserialize(deserializer)?;
    Pubkey::from_str(&pk).map_err(serde::de::Error::custom)
}

fn deserialize_pk_opt<'de, D>(deserializer: D) -> std::result::Result<Option<Pubkey>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(ref value) if !value.is_empty() => Pubkey::from_str(value)
            .map(Some)
            .map_err(serde::de::Error::custom),
        _ => Ok(None),
    }
}

fn serialize_pk_opt<S>(
    pubkey: &Option<Pubkey>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match pubkey {
        Some(pk) => serializer.serialize_some(&pk.to_string()),
        None => serializer.serialize_none(),
    }
}

pub fn serialize_pk<S>(pubkey: &Pubkey, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&pubkey.to_string())
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Debug)]
pub struct UnsupportedAccount {
    pub address: String,
    pub err: String,
}

#[derive(Debug)]
pub struct TokenAccount {
    pub address: Pubkey,
    pub program_id: Pubkey,
    pub is_associated: bool,
    pub account: UiTokenAccount,
    pub has_permanent_delegate: bool,
    pub metadata: TokenMetadata,
}

impl Display for TokenAccount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} ",
            self.metadata,
            self.account.token_amount.real_number_string(),
        )
    }
}

#[derive(Debug)]
pub struct TokenAccounts {
    pub accounts: Vec<TokenAccount>,
    pub unsupported_accounts: Vec<UnsupportedAccount>,
    pub max_len_balance: usize,
    pub aux_len: usize,
    pub explicit_token: bool,
}

impl Display for TokenAccounts {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "accounts:{} unsupported:{} max_len_balance:{} aux_len:{}",
            self.accounts.len(),
            self.unsupported_accounts.len(),
            self.max_len_balance,
            self.aux_len
        )
    }
}
