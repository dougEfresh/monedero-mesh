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

pub struct TokenTransferClient {
    account: Pubkey,
    signer: WalletConnectSigner,
    token: Token<ProgramRpcClientSendTransaction>,
    client: Arc<RpcClient>,
    program_id: Pubkey,
}

impl TokenTransferClient {
    pub async fn init(
        signer: WalletConnectSigner,
        client: Arc<RpcClient>,
        token_address: impl Into<Pubkey>,
        program_id: Pubkey,
    ) -> Result<Self> {
        let token_address = token_address.into();
        let tc: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
            ProgramRpcClient::new(client.clone(), ProgramRpcClientSendTransaction),
        );
        let token = Token::new(
            tc,
            &program_id,
            &token_address,
            None,
            Arc::new(signer.clone()),
        );
        let account = token.get_associated_token_address(&signer.pubkey());
        Ok(Self {
            signer,
            account,
            token,
            client,
            program_id,
        })
    }

    pub fn init_wrap_native(
        signer: WalletConnectSigner,
        client: Arc<RpcClient>,
        program_id: Pubkey,
    ) -> Self {
        let tc: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
            ProgramRpcClient::new(client.clone(), ProgramRpcClientSendTransaction),
        );
        let token = Token::new_native(
            tc,
            &program_id,
            Arc::new(signer.clone()),
        );
        let account = token.get_associated_token_address(&signer.pubkey());
        Self {
            signer,
            account,
            token,
            client,
            program_id,
        }
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
        crate::finish_tx(self.client.clone(), &result).await
    }

    pub async fn mint_to(&self, to: &Pubkey, amount: u64) -> Result<Signature> {
        // TODO optimize to one transaction
        self.token.get_or_create_associated_account_info(to).await?;
        let to_account = self.token.get_associated_token_address(to);
        let result = self.token.mint_to(&to_account, &self.signer.pubkey(), amount, &[&self.signer]).await?;
        crate::finish_tx(self.client.clone(), &result).await
    }

    pub async fn wrap(&self, amount: u64, immutable_owner: bool ) -> Result<Signature> {
        if immutable_owner && self.program_id == spl_token::id() {
            return Err(crate::Error::InvalidTokenProgram);
        }
        if immutable_owner {
            let result = self.token.wrap(&self.account, &self.signer.pubkey(), amount, &[&self.signer]).await?;
            return crate::finish_tx(self.client.clone(), &result).await
        }
        let result = self.token.wrap_with_mutable_ownership(&self.account, &self.signer.pubkey(), amount, &[&self.signer]).await?;
        crate::finish_tx(self.client.clone(), &result).await
    }
}
