use {
    crate::{ReownSigner, Result},
    solana_program::pubkey::Pubkey,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{
        signature::{Keypair, Signature},
        signer::Signer,
    },
    spl_token_client::{
        client::{ProgramClient, ProgramRpcClient, ProgramRpcClientSendTransaction},
        token::{ExtensionInitializationParams, Token},
    },
    std::sync::Arc,
};

pub struct TokenMintClient {
    signer: Arc<ReownSigner>,
    tc: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>>,
    client: Arc<RpcClient>,
}

impl TokenMintClient {
    pub fn new(client: Arc<RpcClient>, signer: Arc<ReownSigner>) -> Self {
        let tc: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
            ProgramRpcClient::new(client.clone(), ProgramRpcClientSendTransaction),
        );

        Self { signer, tc, client }
    }

    pub async fn create_mint(
        &self,
        decimals: u8,
        extension_initialization_params: Option<Vec<ExtensionInitializationParams>>,
    ) -> Result<(Pubkey, Signature)> {
        let token_signer = Keypair::new();
        let token_address = token_signer.pubkey();

        let token = match &extension_initialization_params {
            Some(_) => Token::new(
                self.tc.clone(),
                &spl_token_2022::id(),
                &token_address,
                Some(decimals),
                Arc::new(self.signer.clone()),
            ),
            None => Token::new(
                self.tc.clone(),
                &spl_token::id(),
                &token_address,
                Some(decimals),
                Arc::new(self.signer.clone()),
            ),
        };
        let extensions = extension_initialization_params.unwrap_or_default();
        let wc_signer = Arc::new(self.signer.clone());
        let mut signers: Vec<Arc<dyn Signer>> =
            vec![wc_signer.clone(), wc_signer.clone(), Arc::new(token_signer)];

        signers.push(wc_signer.clone());

        let result = token
            .create_mint(
                &self.signer.pubkey(),
                Some(&self.signer.pubkey()),
                extensions,
                &signers,
            )
            .await?;
        let sig = crate::finish_tx(self.client.clone(), &result).await?;
        Ok((token_address, sig))
    }
}
