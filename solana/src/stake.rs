mod account;
mod client;

use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use solana_program::clock::{Epoch, Slot, UnixTimestamp};
use solana_program::pubkey::Pubkey;
use solana_program::stake::state::{Authorized, Lockup};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::account_utils::StateMut;

use crate::{ReownSigner, SolanaSession};

pub struct StakeClient {
    session: SolanaSession,
    signer: ReownSigner,
    rpc: Arc<RpcClient>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum StakeType {
    Stake,
    RewardsPool,
    Uninitialized,
    Initialized,
}

impl Default for StakeType {
    fn default() -> Self {
        Self::Uninitialized
    }
}

#[derive(Serialize, Debug, Deserialize)]
pub struct StakeLockup {
    pub unix_timestamp: UnixTimestamp,
    pub epoch: Epoch,
    pub custodian: String,
}

impl From<&Lockup> for StakeLockup {
    fn from(lockup: &Lockup) -> Self {
        Self {
            unix_timestamp: lockup.unix_timestamp,
            epoch: lockup.epoch,
            custodian: lockup.custodian.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EpochReward {
    pub epoch: Epoch,
    pub effective_slot: Slot,
    pub amount: u64,       // lamports
    pub post_balance: u64, // lamports
    pub percent_change: f64,
    pub apr: Option<f64>,
    pub commission: Option<u8>,
    pub block_time: UnixTimestamp,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StakeState {
    pub stake_type: StakeType,
    pub account_balance: u64,
    pub credits_observed: u64,
    pub delegated_stake: u64,
    pub delegated_vote_account_address: Option<String>,
    pub activation_epoch: Epoch,
    pub deactivation_epoch: Epoch,
    pub lockup: Option<StakeLockup>,
    pub authorized: Option<Authorized>,
    pub current_epoch: Epoch,
    pub rent_exempt_reserve: u64,
    pub active_stake: u64,
    pub activating_stake: u64,
    pub deactivating_stake: u64,
    pub epoch_rewards: Option<Vec<EpochReward>>,
}

impl Display for StakeState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "delegated:{} balance:{} epoch:{}",
            self.delegated_stake, self.account_balance, self.current_epoch
        )
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyedStakeState {
    pub stake_pubkey: Pubkey,
    #[serde(flatten)]
    pub stake_state: StakeState,
}

impl Display for KeyedStakeState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "account:{} state:{}",
            self.stake_pubkey, self.stake_state
        )
    }
}
