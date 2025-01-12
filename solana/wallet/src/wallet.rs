use {
    monedero_signer_solana::{ReownSigner, SolanaSession},
    solana_pubkey::Pubkey,
    solana_sdk::{
        instruction::Instruction,
        message::Message,
        signature::Signature,
        signer::{Signer, SignerError},
        transaction::Transaction,
    },
    std::{fmt::Display, sync::Arc},
    wallet_standard::{
        SOLANA_SIGN_AND_SEND_TRANSACTION,
        SOLANA_SIGN_IN,
        SOLANA_SIGN_MESSAGE,
        SOLANA_SIGN_TRANSACTION,
        STANDARD_CONNECT,
        STANDARD_DISCONNECT,
        STANDARD_EVENTS,
    },
    wasm_client_solana::SolanaRpcClient as RpcClient,
};

mod memo;
mod transaction;

pub const WALLET_FEATURES: [&str; 7] = [
    STANDARD_CONNECT,
    STANDARD_DISCONNECT,
    STANDARD_EVENTS,
    SOLANA_SIGN_MESSAGE,
    SOLANA_SIGN_IN,
    SOLANA_SIGN_TRANSACTION,
    SOLANA_SIGN_AND_SEND_TRANSACTION,
];

#[derive(Clone)]
pub struct SolanaWallet {
    signer: Arc<ReownSigner>,
    sol_session: SolanaSession,
    pubkey: Pubkey,
    rpc: Arc<RpcClient>,
    memo: Option<String>,
    // fee_service: FeeService,
}

#[derive(Debug)]
pub enum FeeType {
    Units(u32),
    Priority(u64),
}

impl Display for SolanaWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.pk())
    }
}

impl SolanaWallet {
    pub fn new(sol_session: SolanaSession, rpc: Arc<RpcClient>) -> crate::Result<Self> {
        let signer = Arc::new(ReownSigner::new(sol_session.clone()));
        // let fee_service = FeeService::new(sol_session.pubkey(), rpc.clone(),
        // max_fee);
        let pubkey = sol_session.pubkey();
        Ok(Self {
            sol_session,
            signer,
            pubkey,
            rpc,
            memo: None,
            // fee_service,
        })
    }

    pub(super) async fn send_instructions(&self, ix: &[Instruction]) -> crate::Result<Signature> {
        let block = self.rpc.get_latest_blockhash().await?;
        let msg = Message::new_with_blockhash(ix, Some(&self.pubkey), &block);
        let mut tx = Transaction::new_unsigned(msg);
        tx.try_sign(&[&self.signer], tx.message.recent_blockhash)?;
        Ok(self.rpc.send_transaction(&tx.into()).await?)
    }

    pub fn rpc(&self) -> Arc<RpcClient> {
        self.rpc.clone()
    }

    pub fn pk(&self) -> &Pubkey {
        &self.pubkey
    }

    pub async fn balance(&self) -> crate::Result<u64> {
        Ok(self.rpc.get_balance(&self.pubkey).await?)
    }

    // pub async fn fees(&self) -> crate::Result<Vec<FeeType>> {
    //    let mut fees: Vec<FeeType> = Vec::with_capacity(10);
    //    let to = Pubkey::new_unique();
    //    let transfer_ix = self.transfer_instructions(&to, 100);
    //    let fee = self
    //        .fee_service
    //        .simulate(&transfer_ix)
    //        .await?
    //        .unwrap_or_default();
    //    fees.push(FeeType::Units(fee));
    //    let fee = self
    //        .fee_service
    //        .compute_fee()
    //        .await
    //        .ok()
    //        .unwrap_or_default();
    //    fees.push(FeeType::Priority(fee));
    //    Ok(fees)
    //}
    //
}

impl Signer for SolanaWallet {
    fn try_pubkey(&self) -> Result<Pubkey, SignerError> {
        Ok(self.pubkey)
    }

    fn try_sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        self.signer.try_sign_message(message)
    }

    fn is_interactive(&self) -> bool {
        true
    }
}
