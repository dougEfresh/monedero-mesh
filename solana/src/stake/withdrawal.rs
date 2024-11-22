use solana_program::message::Message;
use solana_sdk::signature::Signature;
use solana_sdk::stake::instruction::{self as stake_instruction};
use solana_sdk::transaction::Transaction;

use crate::{KeyedStakeState, Result, StakeClient};

impl StakeClient {
    pub async fn withdraw(&self, account: &KeyedStakeState) -> Result<Signature> {
        let instruction = stake_instruction::withdraw(
            &account.stake_pubkey,
            &self.session.pk,
            &self.session.pk,
            account.stake_state.account_balance,
            None,
        );
        let msg = Message::new(&[instruction], Some(&self.session.pk));
        let hash = self.rpc.get_latest_blockhash().await?;
        let mut tx = Transaction::new_unsigned(msg);
        tx.try_sign(&[&self.signer], hash)?;
        Ok(self.rpc.send_and_confirm_transaction(&tx).await?)
    }
}
