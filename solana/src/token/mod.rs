mod account_client;
mod client;
mod metadata;
mod mint;
mod sort;
mod symbol;

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub use account_client::*;
pub use client::*;
pub use metadata::*;
pub use mint::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use solana_account_decoder::parse_token::UiTokenAccount;
use solana_program::pubkey::Pubkey;
pub use symbol::*;

use crate::Result;

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
        write!(f, "{}", self.metadata.symbol)
    }
}

#[derive(Debug)]
pub struct TokenAccounts {
    pub accounts: BTreeSet<TokenAccount>,
    pub unsupported_accounts: Vec<UnsupportedAccount>,
    pub max_len_balance: usize,
    pub aux_len: usize,
    pub explicit_token: bool,
}

impl PartialEq<Self> for TokenAccount {
    fn eq(&self, other: &Self) -> bool {
        if self.metadata.symbol.eq(&self.metadata.symbol) {
            return self.address == other.address;
        }
        self.metadata.symbol.eq(&other.metadata.symbol)
    }
}

impl PartialOrd<TokenAccount> for TokenAccount {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.metadata.symbol.partial_cmp(&other.metadata.symbol)
    }
}

impl Eq for TokenAccount {}

impl Ord for TokenAccount {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.metadata.symbol.eq(&self.metadata.symbol) {
            return self.address.cmp(&other.address);
        }
        self.metadata.symbol.cmp(&other.metadata.symbol)
    }
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
