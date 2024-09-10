use crate::{Result, SolanaSession, WalletConnectSigner};
use solana_account_decoder::parse_token::UiTokenAmount;
use solana_program::pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use spl_token_2022::extension::StateWithExtensionsOwned;
use spl_token_2022::state::Mint;
use spl_token_client::client::RpcClientResponse;
use spl_token_client::{
    client::{ProgramClient, ProgramRpcClient, ProgramRpcClientSendTransaction},
    token::{ComputeUnitLimit, Token},
};
use std::sync::Arc;

pub struct TokenClientBuilder {
    tc: Arc<Box<dyn ProgramClient<ProgramRpcClientSendTransaction>>>,
    token_address: Pubkey,
    session: SolanaSession,
}

impl TokenClientBuilder {
    pub fn new(
        session: SolanaSession,
        rpc: Arc<RpcClient>,
        token_address: impl Into<Pubkey>,
    ) -> Self {
        let token = token_address.into();
        let tc: Arc<Box<dyn ProgramClient<ProgramRpcClientSendTransaction>>> = Arc::new(Box::new(
            ProgramRpcClient::new(rpc, ProgramRpcClientSendTransaction),
        ));
        Self {
            tc,
            session,
            token_address: token,
        }
    }
}

#[derive(Clone)]
pub(crate) struct MintInfo {
    pub program_id: Pubkey,
    pub mint: StateWithExtensionsOwned<Mint>,
    pub decimals: u8,
    pub address: Pubkey,
}

pub struct TokenTransferClient {
    account: Pubkey,
    signer: WalletConnectSigner,
    token: Token<ProgramRpcClientSendTransaction>,
}

impl TokenTransferClient {
    pub async fn init(
        signer: WalletConnectSigner,
        client: Arc<RpcClient>,
        token_address: impl Into<Pubkey>,
    ) -> Result<Self> {
        let token_address = token_address.into();
        let tc: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
            ProgramRpcClient::new(client, ProgramRpcClientSendTransaction),
        );
        let token = Token::new(
            tc,
            &spl_token::id(),
            &token_address,
            None,
            Arc::new(signer.clone()),
        );
        let account = token.get_associated_token_address(&signer.pubkey());
        Ok(Self {
            signer,
            account,
            token,
        })
    }

    pub fn account(&self) -> &Pubkey {
        &self.account
    }

    pub async fn balance(&self) -> Result<u64> {
        let info = self.token.get_account_info(&self.account).await?;
        Ok(info.base.amount)
    }

    pub async fn transfer(&self, to: &Pubkey, amt: u64) -> Result<Signature> {
        let to_account = self.token.get_associated_token_address(to);
        tracing::info!("destination account {to_account}");
        let result = self
            .token
            .create_recipient_associated_account_and_transfer(
                &self.account,
                &to_account,
                &to,
                &self.signer.pubkey(),
                amt,
                None,
                &[&self.signer],
            )
            .await?;
        match result {
            RpcClientResponse::Signature(sig) => Ok(sig),
            RpcClientResponse::Transaction(t) => {
                tracing::debug!("weird, got back a transaction");
                Err(crate::Error::InvalidRpcResponse)
            }
            RpcClientResponse::Simulation(sim) => Err(crate::Error::InvalidRpcResponse),
        }
    }
}
