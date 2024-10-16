use std::collections::HashMap;
use solana_sdk::pubkey::Pubkey;
use widgetui::State;
use monedero_solana::{KeyedStakeState, StakeState};


pub struct AccountTx {
    pub balance_tx: tokio::sync::watch::Sender<u64>,
}

pub struct AccountRx {
    pub balance_rx: tokio::sync::watch::Receiver<u64>,
}

impl AccountRx {
    pub fn balance(&self) -> Option<u64> {
        if let Some(b) = self.balance_rx.has_changed().ok() {
            if b {
                let balance = *self.balance_rx.borrow();
                return Some(balance);
            }
        }
        None
    }
}

pub struct AccountState {
    pub balance: u64,
    pub stake_accounts: HashMap<Pubkey, StakeState>,
    pub account_rx: AccountRx,
}

impl AccountState {
    pub fn new(balance: u64, stake_accounts: Vec<KeyedStakeState>, account_rx: AccountRx ) -> Self {
        let stake_accounts: HashMap<Pubkey, StakeState> = stake_accounts.into_iter().map(|s| (s.stake_pubkey, s.stake_state)).collect();
        Self {
            balance,
            account_rx,
            stake_accounts,
        }
    }

    pub fn updated_balance(&mut self) -> u64 {
        if let Some(b) = self.account_rx.balance() {
            self.balance = b
        }
        self.balance
    }
}

impl State for AccountState {}
