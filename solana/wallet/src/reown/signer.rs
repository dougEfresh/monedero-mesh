use {super::ReownWallet, solana_pubkey::Pubkey, solana_signature::Signature};

impl wallet_standard::prelude::Signer for ReownWallet {
    fn try_pubkey(&self) -> Result<Pubkey, solana_sdk::signer::SignerError> {
        let Some(ref session) = self.session else {
            return Err(solana_sdk::signer::SignerError::Connection(
                "No connected account".into(),
            ));
        };

        Ok(session.pubkey())
    }

    fn try_sign_message(
        &self,
        _message: &[u8],
    ) -> Result<Signature, solana_sdk::signer::SignerError> {
        let Some(ref _session) = self.session else {
            return Err(solana_sdk::signer::SignerError::Connection(
                "No connected account".into(),
            ));
        };

        Err(solana_sdk::signer::SignerError::Connection(
            "not yet implemented".into(),
        ))
    }

    fn is_interactive(&self) -> bool {
        true
    }
}
