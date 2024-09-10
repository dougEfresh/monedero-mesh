use crate::{Result, WalletConnectSigner};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use spl_token_client::client::{ProgramClient, ProgramRpcClient, ProgramRpcClientSendTransaction};
use spl_token_client::token::Token;
use std::sync::Arc;

pub struct TokenMintClient {
    signer: WalletConnectSigner,
    token: Token<ProgramRpcClientSendTransaction>,
}

impl TokenMintClient {
    pub fn new(client: Arc<RpcClient>, signer: WalletConnectSigner, decimals: u8) -> Self {
        let tc: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
            ProgramRpcClient::new(client, ProgramRpcClientSendTransaction),
        );
        let token = Token::new(
            tc,
            &spl_token::id(),
            &signer.pubkey(),
            None,
            Arc::new(signer.clone()),
        );
        Self { signer, token }
    }

    //pub fn create_mint(&self) -> Result<Signature> {

    //}
}
