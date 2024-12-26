mod account;
mod info;
mod signer;

use {
    account::ReownAccountInfo,
    async_trait::async_trait,
    futures::future::try_join_all,
    info::ReownInfo,
    monedero_signer_solana::{ReownSigner, SolanaSession},
    solana_sdk::{signature::Signature, transaction::VersionedTransaction},
    wallet_standard::{
        prelude::*,
        SolanaSignAndSendTransactionProps,
        SolanaSignTransactionProps,
        StandardConnectInput,
    },
    wasm_client_solana::{SolanaRpcClient as RpcClient, VersionedTransactionExtension},
};

#[derive(Clone)]
pub struct ReownWallet {
    wallet_info: ReownInfo,
    session: Option<SolanaSession>,
    signer: Option<ReownSigner>,
    rpc: RpcClient,
}

impl ReownWallet {
    pub fn new_session(rpc: RpcClient, session: SolanaSession) -> Self {
        let signer = ReownSigner::new(session.clone());
        let accounts = vec![ReownAccountInfo::new_session(&session)];
        let wallet_info = ReownInfo::new_session(accounts);
        Self {
            wallet_info,
            session: Some(session),
            signer: Some(signer),
            rpc,
        }
    }

    pub fn new(rpc: RpcClient) -> Self {
        let wallet_info = ReownInfo::new_session(Vec::new());
        Self {
            wallet_info,
            session: None,
            signer: None,
            rpc,
        }
    }
}

impl Wallet for ReownWallet {
    type Account = ReownAccountInfo;
    type Wallet = ReownInfo;

    fn wallet(&self) -> Self::Wallet {
        self.wallet_info.clone()
    }

    fn wallet_account(&self) -> Option<Self::Account> {
        self.wallet_info.account()
    }
}

#[async_trait(?Send)]
impl WalletStandardDisconnect for ReownWallet {
    async fn disconnect(&mut self) -> WalletResult<()> {
        self.signer = None;
        // if let Some(ref s) = self.session {
        //}
        self.session = None;
        Ok(())
    }
}

#[async_trait(?Send)]
impl WalletStandardConnect for ReownWallet {
    async fn connect(&mut self) -> WalletResult<Vec<Self::Account>> {
        if self.session.is_none() {
            return Err(WalletError::WalletConnection);
        };

        // self.account = Some(account.clone());

        Ok(self.wallet_info.accounts())
    }

    async fn connect_with_options(
        &mut self,
        _: StandardConnectInput,
    ) -> WalletResult<Vec<Self::Account>> {
        self.connect().await
    }
}

#[async_trait(?Send)]
impl WalletSolanaSignTransaction for ReownWallet {
    type Output = VersionedTransaction;

    async fn sign_transaction(
        &self,
        SolanaSignTransactionProps {
            mut transaction, ..
        }: SolanaSignTransactionProps,
    ) -> WalletResult<Self::Output> {
        let Some(ref signer) = self.signer else {
            return Err(WalletError::WalletNotConnected);
        };

        let message_blockhash = *transaction.message.recent_blockhash();

        transaction.try_sign(
            &[&signer],
            if message_blockhash == solana_sdk::hash::Hash::default() {
                Some(self.rpc.get_latest_blockhash().await?)
            } else {
                None
            },
        )?;

        Ok(transaction)
    }

    async fn sign_transactions(
        &self,
        inputs: Vec<SolanaSignTransactionProps>,
    ) -> WalletResult<Vec<Self::Output>> {
        let futures = inputs.into_iter().map(|input| self.sign_transaction(input));

        try_join_all(futures).await
    }
}

pub struct ReownSolanaSignMessageOutput {
    signature: Signature,
    signed_message: Vec<u8>,
}

impl SolanaSignatureOutput for ReownSolanaSignMessageOutput {
    fn try_signature(&self) -> WalletResult<Signature> {
        Ok(self.signature)
    }

    fn signature(&self) -> Signature {
        self.signature
    }
}

impl SolanaSignMessageOutput for ReownSolanaSignMessageOutput {
    fn signed_message(&self) -> Vec<u8> {
        self.signed_message.clone()
    }

    fn signature_type(&self) -> Option<String> {
        Some("Ed25519".into())
    }
}

#[async_trait(?Send)]
impl WalletSolanaSignMessage for ReownWallet {
    type Output = ReownSolanaSignMessageOutput;

    async fn sign_message_async(&self, message: impl Into<Vec<u8>>) -> WalletResult<Self::Output> {
        let Some(ref signer) = self.session else {
            return Err(WalletError::WalletNotConnected);
        };

        let signed_message = message.into();
        let message = bs58::encode(&signed_message).into_string();
        let payload = monedero_signer_solana::SignMessageRequest {
            pubkey: signer.pubkey(),
            message,
        };
        let signature = signer
            .sign_message_payload(payload)
            .await
            .map_err(|e| WalletError::Signer(format!("{e:?}")))?;

        Ok(ReownSolanaSignMessageOutput {
            signature,
            signed_message,
        })
    }

    /// Sign a list of messages using the account's secret key.
    async fn sign_messages<M: Into<Vec<u8>>>(
        &self,
        messages: Vec<M>,
    ) -> WalletResult<Vec<Self::Output>> {
        let futures = messages
            .into_iter()
            .map(|message| WalletSolanaSignMessage::sign_message_async(self, message));

        try_join_all(futures).await
    }
}

#[async_trait(?Send)]
impl WalletSolanaSignAndSendTransaction for ReownWallet {
    type Output = Signature;

    async fn sign_and_send_transaction(
        &self,
        SolanaSignAndSendTransactionProps {
            mut transaction, ..
        }: SolanaSignAndSendTransactionProps,
    ) -> WalletResult<Self::Output> {
        let Some(ref signer) = self.signer else {
            return Err(WalletError::WalletNotConnected);
        };

        let message_blockhash = *transaction.message.recent_blockhash();

        transaction.try_sign(
            &[signer],
            if message_blockhash == solana_sdk::hash::Hash::default() {
                Some(self.rpc.get_latest_blockhash().await?)
            } else {
                None
            },
        )?;
        let signature = self.rpc.send_transaction(&transaction).await?;

        Ok(signature)
    }

    async fn sign_and_send_transactions(
        &self,
        inputs: Vec<SolanaSignAndSendTransactionProps>,
    ) -> WalletResult<Vec<Self::Output>> {
        let futures = inputs
            .into_iter()
            .map(|input| self.sign_and_send_transaction(input));

        try_join_all(futures).await
    }
}
