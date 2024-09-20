use crate::stake::{KeyedStakeState, StakeClient, StakeState};
use crate::{SolanaSession, WalletConnectSigner};
use solana_program::clock::{Clock, Epoch, Slot};
use solana_program::feature::Feature;
use solana_program::pubkey::Pubkey;
use solana_program::stake::state::{Meta, StakeActivationStatus, StakeStateV2};
use solana_program::stake_history::StakeHistory;
use solana_program::sysvar::{self, stake_history};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_rpc_client_api::filter::{Memcmp, RpcFilterType};
use solana_rpc_client_api::response::RpcVoteAccountInfo;
use solana_sdk::account::{from_account, ReadableAccount};
use solana_sdk::account_utils::StateMut;
use std::str::FromStr;
use std::sync::Arc;

async fn get_feature_activation_slot(
    rpc: &RpcClient,
    feature_id: &Pubkey,
) -> crate::Result<Option<Slot>> {
    let feature_account = rpc.get_account(feature_id).await?;
    let decoded: Feature = bincode::deserialize(feature_account.data())?;
    Ok(decoded.activated_at)
}

impl StakeClient {
    pub fn new(sol: SolanaSession, signer: WalletConnectSigner, rpc: Arc<RpcClient>) -> Self {
        Self {
            session: sol,
            signer,
            rpc,
        }
    }

    pub async fn validators(&self) -> crate::Result<Vec<RpcVoteAccountInfo>> {
        let delegators = self.rpc.get_vote_accounts().await?;
        Ok(delegators.current)
    }

    pub async fn accounts(&self) -> crate::Result<Vec<KeyedStakeState>> {
        let id = solana_sdk::stake::program::id();
        let program_accounts_config = RpcProgramAccountsConfig {
            account_config: RpcAccountInfoConfig {
                encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                44,
                self.session.pubkey().as_ref(),
            ))]),
            ..RpcProgramAccountsConfig::default()
        };
        let all_stake_accounts = self
            .rpc
            .get_program_accounts_with_config(&id, program_accounts_config)
            .await?;
        let stake_history_account = self.rpc.get_account(&stake_history::id()).await?;
        let clock_account = self.rpc.get_account(&sysvar::clock::id()).await?;
        let clock: Clock = from_account(&clock_account).ok_or(crate::Error::RpcRequestError(
            "Failed to deserialize clock sysvar".to_string(),
        ))?;
        let stake_history: StakeHistory = from_account(&stake_history_account).ok_or(
            crate::Error::RpcRequestError("Failed to deserialize state history".to_string()),
        )?;

        let feature_set_id = Pubkey::from_str("GwtDQBghCTBgmX2cpEGNPxTEBUTQRaDMGTr5qychdGMj")?;
        let new_rate_activation_epoch =
            get_feature_activation_slot(&self.rpc, &feature_set_id).await?;

        let mut stake_accounts: Vec<KeyedStakeState> = vec![];
        for (stake_pubkey, stake_account) in all_stake_accounts {
            let stake_state = stake_account.state()?;
            match stake_state {
                StakeStateV2::Initialized(_) | StakeStateV2::Stake(_, _, _) => {
                    stake_accounts.push(KeyedStakeState {
                        stake_pubkey: stake_pubkey.to_string(),
                        stake_state: build_stake_state(
                            stake_account.lamports,
                            &stake_state,
                            &stake_history,
                            &clock,
                            new_rate_activation_epoch,
                        ),
                    });
                }
                _ => {}
            }
        }
        Ok(stake_accounts)
    }
}

fn build_stake_state(
    account_balance: u64,
    stake_state: &StakeStateV2,
    stake_history: &StakeHistory,
    clock: &Clock,
    new_rate_activation_epoch: Option<Epoch>,
) -> StakeState {
    match stake_state {
        StakeStateV2::Stake(
            Meta {
                rent_exempt_reserve,
                authorized,
                lockup,
            },
            stake,
            _,
        ) => {
            let current_epoch = clock.epoch;
            let StakeActivationStatus {
                effective,
                activating,
                deactivating,
            } = stake.delegation.stake_activating_and_deactivating(
                current_epoch,
                stake_history,
                new_rate_activation_epoch,
            );
            let lockup = if lockup.is_in_force(clock, None) {
                Some(lockup.into())
            } else {
                None
            };
            StakeState {
                stake_type: super::StakeType::Stake,
                account_balance,
                credits_observed: stake.credits_observed,
                delegated_stake: stake.delegation.stake,
                delegated_vote_account_address: if stake.delegation.voter_pubkey
                    != Pubkey::default()
                {
                    Some(stake.delegation.voter_pubkey.to_string())
                } else {
                    None
                },
                activation_epoch: if stake.delegation.activation_epoch < u64::MAX {
                    stake.delegation.activation_epoch
                } else {
                    0
                },
                deactivation_epoch: if stake.delegation.deactivation_epoch < u64::MAX {
                    stake.delegation.deactivation_epoch
                } else {
                    0
                },
                lockup,
                current_epoch,
                rent_exempt_reserve: *rent_exempt_reserve,
                active_stake: effective,
                activating_stake: activating,
                deactivating_stake: deactivating,
                ..StakeState::default()
            }
        }
        StakeStateV2::RewardsPool => StakeState {
            stake_type: super::StakeType::RewardsPool,
            account_balance,
            ..StakeState::default()
        },
        StakeStateV2::Uninitialized => StakeState {
            account_balance,
            ..StakeState::default()
        },
        StakeStateV2::Initialized(Meta {
            rent_exempt_reserve,
            authorized,
            lockup,
        }) => {
            let lockup = if lockup.is_in_force(clock, None) {
                Some(lockup.into())
            } else {
                None
            };
            StakeState {
                stake_type: super::StakeType::Initialized,
                account_balance,
                credits_observed: 0,
                authorized: Some(authorized.clone()),
                lockup,
                rent_exempt_reserve: *rent_exempt_reserve,
                ..StakeState::default()
            }
        }
    }
}
