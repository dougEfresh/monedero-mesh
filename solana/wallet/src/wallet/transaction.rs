use {
    super::SolanaWallet,
    solana_pubkey::Pubkey,
    solana_sdk::{instruction::Instruction, message::Message, transaction::Transaction},
    solana_signature::Signature,
};

impl SolanaWallet {
    pub async fn transfer(&self, to: &Pubkey, lamports: u64) -> crate::Result<Signature> {
        let ix = self.transfer_instructions(to, lamports);
        let block = self.rpc.get_latest_blockhash().await?;
        let msg = Message::new_with_blockhash(&ix, Some(&self.pubkey), &block);
        let mut tx = Transaction::new_unsigned(msg);
        tracing::info!("message {:?}", tx.message());
        tx.try_sign(&[&self.signer], tx.message.recent_blockhash)?;
        Ok(self.rpc.send_transaction(&tx.into()).await?)
    }

    fn transfer_instructions(&self, to: &Pubkey, lamports: u64) -> Vec<Instruction> {
        vec![
            // spl_memo::build_memo(&self.memo, &[&self.sol_session.pk]),
            solana_sdk::system_instruction::transfer(&self.pubkey, to, lamports),
        ]
        //    //.with_memo(Some(&self.memo))
    }
}
