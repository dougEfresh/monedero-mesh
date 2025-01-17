use {
    monedero_signer_solana::{ReownSigner, SolanaSession},
    solana_pubkey::Pubkey,
    solana_sdk::{
        address_lookup_table::AddressLookupTableAccount,
        instruction::Instruction,
        message::{v0::Message, VersionedMessage},
        signature::Signature,
        signer::{Signer, SignerError},
        transaction::VersionedTransaction,
    },
    std::{
        fmt::{Debug, Display},
        sync::Arc,
    },
    tracing::Level,
    wallet_standard::{
        SOLANA_SIGN_AND_SEND_TRANSACTION,
        SOLANA_SIGN_IN,
        SOLANA_SIGN_MESSAGE,
        SOLANA_SIGN_TRANSACTION,
        STANDARD_CONNECT,
        STANDARD_DISCONNECT,
        STANDARD_EVENTS,
    },
    wasm_client_solana::{
        solana_transaction_status::UiTransactionEncoding,
        RpcSimulateTransactionConfig,
        SimulateTransactionResponseValue,
        SolanaRpcClient as RpcClient,
        VersionedTransactionExtension,
    },
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

impl Debug for SolanaWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.pk())
    }
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

    #[tracing::instrument(level = "info", skip(table))]
    async fn lookup(
        &self,
        table: Option<&Pubkey>,
    ) -> crate::Result<Vec<AddressLookupTableAccount>> {
        match table {
            None => Ok(Vec::new()),
            Some(pubkey) => {
                let addr_table = self
                    .rpc
                    .get_address_lookup_table(pubkey)
                    .await?
                    .optional_address_lookup_table_account(pubkey)
                    .ok_or(crate::Error::UninitializedLookupTable(pubkey.to_string()))?;
                Ok(vec![addr_table])
            }
        }
    }

    pub async fn simulate(
        &self,
        tx: &VersionedTransaction,
    ) -> crate::Result<SimulateTransactionResponseValue> {
        let span = tracing::span!(
            Level::INFO,
            "simulate",
            wallet = format!("{self}"),
            computeUnits = 0,
        );
        let _ctx = span.enter();
        let r = self
            .rpc
            .simulate_transaction_with_config(&tx, RpcSimulateTransactionConfig {
                sig_verify: false,
                encoding: Some(UiTransactionEncoding::Base64),
                replace_recent_blockhash: Some(false),
                ..Default::default()
            })
            .await?
            .value;
        r.units_consumed.inspect(|u| {
            span.record("computeUnits", u);
        });
        Ok(r)
    }

    pub async fn send_instructions(
        &self,
        ix: &[Instruction],
        table: Option<&Pubkey>,
    ) -> crate::Result<Signature> {
        let block = self.rpc.get_latest_blockhash().await?;
        let address_lookup = self.lookup(table).await?;
        let instructions = Vec::from(ix);
        let message = VersionedMessage::V0(Message::try_compile(
            self.pk(),
            &instructions,
            &address_lookup,
            block,
        )?);
        let mut tx = VersionedTransaction::new_unsigned(message);
        self.simulate(&tx).await?.units_consumed;
        tx.try_sign(&[self], None)?;
        Ok(self.rpc.send_transaction(&tx).await?)
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
