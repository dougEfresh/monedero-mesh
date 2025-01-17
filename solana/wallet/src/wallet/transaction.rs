use {
    super::SolanaWallet,
    solana_pubkey::Pubkey,
    solana_sdk::instruction::Instruction,
    solana_signature::Signature,
};

impl SolanaWallet {
    pub async fn transfer(&self, to: &Pubkey, lamports: u64) -> crate::Result<Signature> {
        let ix = self.transfer_instructions(to, lamports);
        self.send_instructions(&ix, None).await
    }

    fn transfer_instructions(&self, to: &Pubkey, lamports: u64) -> Vec<Instruction> {
        vec![
            // spl_memo::build_memo(&self.memo, &[&self.sol_session.pk]),
            solana_sdk::system_instruction::transfer(&self.pubkey, to, lamports),
        ]
        //    //.with_memo(Some(&self.memo))
    }
}
