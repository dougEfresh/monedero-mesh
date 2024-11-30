use {
    crate::{KeyedStakeState, Result, StakeClient},
    solana_program::message::Message,
    solana_sdk::{
        signature::Signature,
        stake::instruction::{self as stake_instruction},
        transaction::Transaction,
    },
};

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
