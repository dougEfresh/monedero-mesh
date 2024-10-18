#![allow(clippy::arithmetic_side_effects)]

use std::collections::BTreeSet;
use std::fmt::{Debug, Formatter};
use std::str::FromStr;
use std::sync::Arc;

use solana_account_decoder::parse_token::TokenAccountType;
use solana_account_decoder::UiAccountData;
use solana_program::pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::request::TokenAccountsFilter;
use solana_rpc_client_api::response::RpcKeyedAccount;
use spl_associated_token_account_client::address::get_associated_token_address_with_program_id;

use crate::token::metadata::TokenMetadataClient;
use crate::token::UnsupportedAccount;
use crate::{TokenAccount, TokenAccounts};

pub struct TokenAccountsClient {
    client: Arc<RpcClient>,
    pub(in crate::token) owner: Pubkey,
    pub(in crate::token) metadata_client: TokenMetadataClient,
}

impl Debug for TokenAccountsClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "owner={}", self.owner)
    }
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

    #[tracing::instrument(level = "info")]
    pub async fn accounts(&self) -> crate::Result<TokenAccounts> {
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

impl TokenAccountsClient {
    pub(crate) fn sort_and_parse_token_accounts(
        &self,
        keyed_accounts: Vec<RpcKeyedAccount>,
        explicit_token: bool,
    ) -> crate::Result<TokenAccounts> {
        let mut accounts: BTreeSet<TokenAccount> = BTreeSet::new();
        let mut unsupported_accounts = vec![];
        let mut max_len_balance = 0;
        let mut aux_count = 0;

        for keyed_account in keyed_accounts {
            let address_str = keyed_account.pubkey;
            let address = Pubkey::from_str(&address_str)?;
            let program_id = Pubkey::from_str(&keyed_account.account.owner)?;

            if let UiAccountData::Json(parsed_account) = keyed_account.account.data {
                match serde_json::from_value(parsed_account.parsed) {
                    Ok(TokenAccountType::Account(ui_token_account)) => {
                        let mint = Pubkey::from_str(&ui_token_account.mint)?;
                        let is_associated = get_associated_token_address_with_program_id(
                            &self.owner,
                            &mint,
                            &program_id,
                        ) == address;

                        if !is_associated {
                            aux_count += 1;
                        }

                        max_len_balance = max_len_balance.max(
                            ui_token_account
                                .token_amount
                                .real_number_string_trimmed()
                                .len(),
                        );
                        let metadata = self.metadata_client.get(&ui_token_account);
                        let account = TokenAccount {
                            address,
                            program_id,
                            account: ui_token_account,
                            is_associated,
                            has_permanent_delegate: false,
                            metadata,
                        };
                        accounts.insert(account);
                    }
                    Ok(_) => unsupported_accounts.push(UnsupportedAccount {
                        address: address_str,
                        err: "Not a token account".to_string(),
                    }),
                    Err(err) => unsupported_accounts.push(UnsupportedAccount {
                        address: address_str,
                        err: format!("Account parse failure: {}", err),
                    }),
                }
            }
        }

        Ok(TokenAccounts {
            accounts,
            unsupported_accounts,
            max_len_balance,
            aux_len: if aux_count > 0 {
                format!("  (Aux-{}*)", aux_count).chars().count() + 1
            } else {
                0
            },
            explicit_token,
        })
    }
}
