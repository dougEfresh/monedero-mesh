use {
    crate::{bytes_to_str, ReownSigner, Result, TokenAccount},
    solana_program::pubkey::Pubkey,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{signature::Signature, signer::Signer},
    spl_token_client::{
        client::{ProgramClient, ProgramRpcClient, ProgramRpcClientSendTransaction},
        token::{ComputeUnitLimit, Token},
    },
    std::{
        fmt::{Debug, Display, Formatter},
        sync::Arc,
    },
};

#[derive(Clone)]
pub struct TokenTransferClient {
    account: Pubkey,
    token_address: Pubkey,
    signer: Arc<ReownSigner>,
    token: Arc<Token<ProgramRpcClientSendTransaction>>,
    client: Arc<RpcClient>,
    program_id: Pubkey,
}

impl PartialEq for TokenTransferClient {
    fn eq(&self, other: &Self) -> bool {
        self.account.eq(&other.account) && self.program_id.eq(&other.program_id)
    }
}

impl Debug for TokenTransferClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt_common(f)
    }
}

impl Display for TokenTransferClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt_common(f)
    }
}

impl TokenTransferClient {
    pub fn fmt_common(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "account={} {} program={} token={}",
            self.account, self.signer, self.program_id, self.token_address
        )
    }

    pub fn new(
        signer: Arc<ReownSigner>,
        client: Arc<RpcClient>,
        token_account: &TokenAccount,
        memo: &str,
    ) -> Self {
        let tc: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
            ProgramRpcClient::new(client.clone(), ProgramRpcClientSendTransaction),
        );
        let token = Token::new(
            tc,
            &token_account.program_id,
            &token_account.metadata.address,
            Some(token_account.account.token_amount.decimals),
            signer.clone(),
        )
        .with_compute_unit_limit(ComputeUnitLimit::Simulated);
        token.with_memo(memo, vec![signer.pubkey()]);

        Self {
            token_address: token_account.address,
            signer,
            account: token_account.address,
            token: Arc::new(token),
            client,
            program_id: token_account.program_id,
        }
    }

    pub async fn init(
        signer: Arc<ReownSigner>,
        client: Arc<RpcClient>,
        token_address: impl Into<Pubkey>,
        program_id: Pubkey,
        memo: &[u8],
    ) -> Result<Self> {
        let token_address = token_address.into();
        let tc: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
            ProgramRpcClient::new(client.clone(), ProgramRpcClientSendTransaction),
        );
        let token = Token::new(tc, &program_id, &token_address, None, signer.clone());
        let account = token.get_associated_token_address(&signer.pubkey());
        token.with_memo(bytes_to_str(memo), vec![signer.pubkey()]);
        Ok(Self {
            token_address,
            signer,
            account,
            token: Arc::new(token),
            client,
            program_id,
        })
    }

    pub fn init_wrap_native(
        signer: Arc<ReownSigner>,
        client: Arc<RpcClient>,
        program_id: Pubkey,
        memo: &str,
    ) -> Self {
        let tc: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
            ProgramRpcClient::new(client.clone(), ProgramRpcClientSendTransaction),
        );
        let token = Token::new_native(tc, &program_id, Arc::new(signer.clone()));
        let account = token.get_associated_token_address(&signer.pubkey());
        token.with_memo(memo, vec![signer.pubkey()]);
        Self {
            token_address: Pubkey::default(),
            signer,
            account,
            token: Arc::new(token),
            client,
            program_id,
        }
    }

    pub fn account(&self) -> &Pubkey {
        &self.account
    }

    #[tracing::instrument(level = "info")]
    pub async fn balance(&self) -> Result<u64> {
        let info = self.token.get_account_info(&self.account).await?;

        Ok(info.base.amount)
    }

    #[tracing::instrument(level = "info")]
    pub async fn transfer(&self, to: &Pubkey, amt: u64) -> Result<Signature> {
        let to_account = self.token.get_associated_token_address(to);
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

    #[tracing::instrument(level = "info")]
    pub async fn mint_to(&self, to: &Pubkey, amount: u64) -> Result<Signature> {
        // TODO optimize to one transaction
        self.token.get_or_create_associated_account_info(to).await?;
        let to_account = self.token.get_associated_token_address(to);
        let result = self
            .token
            .mint_to(&to_account, &self.signer.pubkey(), amount, &[&self.signer])
            .await?;
        crate::finish_tx(self.client.clone(), &result).await
    }

    #[tracing::instrument(level = "info")]
    pub async fn wrap(&self, amount: u64, immutable_owner: bool) -> Result<Signature> {
        if immutable_owner && self.program_id == spl_token::id() {
            return Err(crate::Error::InvalidTokenProgram);
        }
        if immutable_owner {
            let result = self
                .token
                .wrap(
                    &self.account,
                    &self.signer.pubkey(),
                    amount,
                    &[&self.signer],
                )
                .await?;
            return crate::finish_tx(self.client.clone(), &result).await;
        }
        let result = self
            .token
            .wrap_with_mutable_ownership(&self.account, &self.signer.pubkey(), amount, &[
                &self.signer
            ])
            .await?;
        crate::finish_tx(self.client.clone(), &result).await
    }
}
