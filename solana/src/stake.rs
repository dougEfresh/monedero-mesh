mod account;
mod client;
mod delegate;
mod withdrawal;

use {
    crate::{fee::FeeService, ReownSigner, SolanaSession},
    serde::{Deserialize, Serialize},
    solana_program::{
        clock::{Epoch, Slot, UnixTimestamp},
        native_token::LAMPORTS_PER_SOL,
        pubkey::Pubkey,
        stake::state::{Authorized, Lockup},
    },
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::account_utils::StateMut,
    std::{
        fmt::{Debug, Display, Formatter},
        sync::Arc,
    },
};

pub struct StakeClient {
    session: SolanaSession,
    signer: ReownSigner,
    rpc: Arc<RpcClient>,
    memo: String,
    fee_service: FeeService,
}

impl Debug for StakeClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[StakeClient][{}]", self.session)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum StakeType {
    Stake,
    RewardsPool,
    Uninitialized,
    Initialized,
}

impl Display for StakeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StakeType::Stake => write!(f, "{}", "Stake"),
            StakeType::RewardsPool => write!(f, "{}", "RewardsPool"),
            StakeType::Uninitialized => write!(f, "{}", "Uninitialized"),
            StakeType::Initialized => write!(f, "{}", "Initialized"),
        }
    }
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
        let v = match &self.delegated_vote_account_address {
            None => String::new(),
            Some(address) => format!(" vote:{}", address),
        };
        write!(
            f,
            "delegated:{} balance:{} type:{} {}",
            self.delegated_stake,
            self.account_balance as f64 / LAMPORTS_PER_SOL as f64,
            self.stake_type,
            v,
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
        write!(f, "{} {}", self.stake_pubkey, self.stake_state)
    }
}
