use std::path::PathBuf;
use std::sync::Arc;

use solana_program::instruction::Instruction;
use solana_program::message::Message;
use solana_program::pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_sdk::signature::Signature;
use solana_sdk::transaction::Transaction;

use crate::fee::FeeService;
use crate::{
    ReownSigner, SolanaSession, StakeClient, TokenAccount, TokenAccountsClient,
    TokenMetadataClient, TokenMintClient, TokenTransferClient, WithMemo, DEFAULT_MEMO,
};

#[derive(Clone)]
pub struct SolanaWallet {
    signer: Arc<ReownSigner>,
    sol_session: SolanaSession,
    rpc: Arc<RpcClient>,
    token_accounts_client: Arc<TokenAccountsClient>,
    memo: String,
    fee_service: FeeService,
}

#[derive(Debug)]
pub enum FeeType {
    Units(u32),
    Priority(u64),
}

/*
impl From<FeeType> for stanza::table::Cell {
    fn from(value: FeeType) -> Self {
        stanza::table::Cell::new()
    }
}
 */

impl SolanaWallet {
    pub async fn init(
        sol_session: SolanaSession,
        rpc: Arc<RpcClient>,
        storage_path: PathBuf,
        max_fee: u64,
        memo: Option<&str>,
    ) -> crate::Result<Self> {
        let signer = Arc::new(ReownSigner::new(sol_session.clone()));
        let metadata_client = TokenMetadataClient::init(storage_path).await?;
        let memo = if let Some(memo) = memo {
            String::from(memo)
        } else {
            String::from(DEFAULT_MEMO)
        };
        let tc = TokenAccountsClient::new(sol_session.pubkey(), rpc.clone(), metadata_client);
        let fee_service = FeeService::new(sol_session.pubkey(), rpc.clone(), max_fee);
        Ok(Self {
            sol_session,
            signer,
            rpc,
            token_accounts_client: Arc::new(tc),
            memo,
            fee_service,
        })
    }

    pub async fn transfer(&self, to: &Pubkey, lamports: u64) -> crate::Result<Signature> {
        let ix = self.transfer_instructions(to, lamports);
        let message = Message::new(&ix, Some(&self.sol_session.pk));
        let block = self.rpc.get_latest_blockhash().await?;
        let tx = Transaction::new(&[&self.signer], message, block);
        Ok(self
            .rpc
            .send_and_confirm_transaction_with_spinner_and_commitment(
                &tx,
                CommitmentConfig {
                    commitment: CommitmentLevel::Finalized,
                },
            )
            .await?)
    }

    fn transfer_instructions(&self, to: &Pubkey, lamports: u64) -> Vec<Instruction> {
        vec![
            //spl_memo::build_memo(&self.memo, &[&self.sol_session.pk]),
            solana_sdk::system_instruction::transfer(&self.sol_session.pk, &to, lamports),
        ]
        .with_memo(Some(&self.memo))
    }

    pub fn stake_client(&self) -> StakeClient {
        StakeClient::new(
            self.sol_session.clone(),
            self.signer.as_ref().clone(),
            self.rpc.clone(),
            self.memo.clone(),
            self.fee_service.clone(),
        )
    }

    pub fn token_accounts_client(&self) -> Arc<TokenAccountsClient> {
        self.token_accounts_client.clone()
    }

    pub fn token_mint_client(&self) -> TokenMintClient {
        TokenMintClient::new(self.rpc.clone(), self.signer.clone())
    }

    pub fn token_wrapped_client(&self) -> TokenTransferClient {
        TokenTransferClient::init_wrap_native(
            self.signer.clone(),
            self.rpc.clone(),
            spl_token::id(),
            &self.memo,
        )
    }
    pub fn token_transfer_client(&self, token: &TokenAccount) -> TokenTransferClient {
        TokenTransferClient::new(self.signer.clone(), self.rpc.clone(), token, &self.memo)
    }

    pub async fn compute_fee(&self) -> crate::Result<u64> {
        Ok(self.fee_service.compute_fee().await?)
    }

    pub async fn fees(&self) -> crate::Result<Vec<FeeType>> {
        let mut fees: Vec<FeeType> = Vec::with_capacity(10);
        let to = Pubkey::new_unique();
        let transfer_ix = self.transfer_instructions(&to, 100);
        let fee = self
            .fee_service
            .simulate(&transfer_ix)
            .await?
            .unwrap_or_default();
        fees.push(FeeType::Units(fee));
        let fee = self
            .fee_service
            .compute_fee()
            .await
            .ok()
            .unwrap_or_default();
        fees.push(FeeType::Priority(fee));
        Ok(fees)
    }

    pub fn pk(&self) -> &Pubkey {
        &self.sol_session.pk
    }

    pub async fn balance(&self) -> crate::Result<u64> {
        Ok(self.rpc.get_balance(self.pk()).await?)
    }
}
