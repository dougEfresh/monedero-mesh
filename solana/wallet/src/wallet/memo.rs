use {
    super::SolanaWallet,
    crate::Result,
    solana_sdk::instruction::Instruction,
    solana_signature::Signature,
    spl_memo::id,
};

impl SolanaWallet {
    pub async fn memo(&self, message: &str) -> Result<Signature> {
        let memo_ix = Instruction {
            program_id: id(),
            accounts: vec![],
            data: message.as_bytes().to_vec(),
        };
        self.send_instructions(&[memo_ix]).await
    }
}
