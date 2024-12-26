use {
    crate::{
        domain::namespaces::{
            Account,
            Accounts,
            Chains,
            EipMethod,
            Events,
            Method,
            Methods,
            Namespace,
            NamespaceName,
            Namespaces,
            SolanaMethod,
        },
        session::{
            SdkErrors,
            SessionProposeRequest,
            SessionRequestRequest,
            WalletRequestResponse,
            WalletSettlementHandler,
        },
        Error,
        SignMessageRequest,
        SolanaSignatureResponse,
        WalletConnectTransaction,
    },
    async_trait::async_trait,
    base64::{prelude::BASE64_STANDARD, Engine},
    solana_sdk::{
        signer::{keypair::Keypair, Signer},
        transaction::Transaction,
    },
    solana_signature::Signature,
    std::collections::{BTreeMap, BTreeSet},
    tracing::info,
};

#[derive(Clone)]
pub struct MockWallet {}

pub const SUPPORTED_ACCOUNT: &str = "215r9xfTFVYcE9g3fAUGowauM84egyUvFCbSo3LKNaep";

#[async_trait]
impl WalletSettlementHandler for MockWallet {
    async fn settlement(
        &self,
        proposal: SessionProposeRequest,
    ) -> monedero_mesh::Result<Namespaces> {
        let mut settled: Namespaces = Namespaces(BTreeMap::new());
        for (name, namespace) in proposal.required_namespaces.iter() {
            let accounts: BTreeSet<Account> = namespace
                .chains
                .iter()
                .map(|c| Account {
                    address: String::from(SUPPORTED_ACCOUNT),
                    chain: c.clone(),
                })
                .collect();

            let methods = match name {
                NamespaceName::EIP155 => EipMethod::defaults(),
                NamespaceName::Solana => SolanaMethod::defaults(),
                NamespaceName::Other(_) => BTreeSet::from([Method::Other("unknown".to_owned())]),
            };
            settled.insert(name.clone(), Namespace {
                accounts: Accounts(accounts),
                chains: Chains(namespace.chains.iter().cloned().collect()),
                methods: Methods(methods),
                events: Events::default(),
            });
        }
        Ok(settled)
    }
}

impl monedero_mesh::SessionEventHandler for MockWallet {}

pub const KEYPAIR: [u8; 64] = [
    186, 128, 232, 61, 254, 246, 197, 13, 125, 103, 212, 83, 16, 121, 144, 20, 93, 161, 35, 128,
    89, 135, 157, 200, 81, 159, 83, 204, 204, 130, 28, 42, 14, 225, 43, 2, 44, 16, 255, 214, 161,
    18, 184, 164, 253, 126, 16, 187, 134, 176, 75, 35, 179, 194, 181, 150, 67, 236, 131, 49, 45,
    155, 45, 253,
];

impl MockWallet {
    pub fn sign_transaction(msg: &[u8], kp: &Keypair) -> crate::Result<Signature> {
        info!("decoding transaction");
        let mut tx = bincode::deserialize::<Transaction>(msg)?;
        info!("tx message is {:?}", tx.message());
        let positions = tx.get_signing_keypair_positions(&[kp.pubkey()])?;
        if positions.is_empty() {
            return Err(crate::Error::NoSigners("something".to_string()));
        }
        tx.try_sign(&[kp], tx.message().recent_blockhash)?;
        info!(
            "tx is signed? '{}' number of sigs '{}' positing '{:?}'",
            tx.is_signed(),
            tx.signatures.len(),
            positions
        );
        if !tx.is_signed() {
            return Err(crate::Error::TransactionNotSigned);
        }
        Ok(tx.signatures[0])
    }

    pub fn sign(
        method: SolanaMethod,
        value: serde_json::Value,
    ) -> crate::Result<SolanaSignatureResponse> {
        let kp = Keypair::from_bytes(&KEYPAIR).map_err(|_| Error::KeyPairFailure)?;
        info!("PK of signer: {}", kp.pubkey());
        let sig: Signature = match method {
            SolanaMethod::SignTransaction => {
                let req = serde_json::from_value::<WalletConnectTransaction>(value)?;
                let msg = BASE64_STANDARD.decode(req.transaction)?;
                Self::sign_transaction(&msg, &kp)?
            }
            SolanaMethod::SignMessage => {
                let req = serde_json::from_value::<SignMessageRequest>(value)?;
                let decoded = bs58::decode(req.message).into_vec()?;
                kp.sign_message(&decoded)
            }
            SolanaMethod::Other(m) => return Err(Error::InvalidMethod(m)),
        };
        let signature: SolanaSignatureResponse = sig.into();
        info!("returning sig: {signature}");
        Ok(signature)
    }
}

#[async_trait]
impl monedero_mesh::SessionHandler for MockWallet {
    async fn request(&self, request: SessionRequestRequest) -> WalletRequestResponse {
        match request.request.method {
            Method::Solana(SolanaMethod::SignTransaction) => {
                match Self::sign(SolanaMethod::SignTransaction, request.request.params) {
                    Err(e) => {
                        tracing::warn!("failed sig: {e}");
                        WalletRequestResponse::Error(SdkErrors::UserRejected)
                    }
                    Ok(sig) => WalletRequestResponse::Success(serde_json::to_value(&sig).unwrap()),
                }
            }
            Method::Solana(SolanaMethod::SignMessage) => {
                match Self::sign(SolanaMethod::SignMessage, request.request.params) {
                    Err(e) => {
                        tracing::warn!("failed signing message: {e}");
                        WalletRequestResponse::Error(SdkErrors::UserRejected)
                    }
                    Ok(sig) => WalletRequestResponse::Success(serde_json::to_value(&sig).unwrap()),
                }
            }
            _ => WalletRequestResponse::Error(SdkErrors::InvalidMethod),
        }
    }
}
