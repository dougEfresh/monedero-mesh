use {
    async_trait::async_trait,
    base64::{prelude::BASE64_STANDARD, Engine},
    monedero_signer_solana::{
        domain::namespaces::{
            Account, Accounts, Chains, EipMethod, Events, Method, Methods, Namespace,
            NamespaceName, Namespaces, SolanaMethod,
        },
        session::{
            SdkErrors, SessionProposeRequest, SessionRequestRequest, WalletRequestResponse,
            WalletSettlementHandler,
        },
        Dapp, Error, SignMessageRequest, SolanaSignatureResponse, WalletConnectTransaction,
    },
    solana_sdk::signer::{keypair::Keypair, Signer},
    std::collections::{BTreeMap, BTreeSet},
    tracing::info,
};

#[allow(dead_code)]
pub struct TestContext {
    pub dapp: Dapp,
    pub session: monedero_signer_solana::SolanaSession,
    pub wallet: MockWallet,
}

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
            settled.insert(
                name.clone(),
                Namespace {
                    accounts: Accounts(accounts),
                    chains: Chains(namespace.chains.iter().cloned().collect()),
                    methods: Methods(methods),
                    events: Events::default(),
                },
            );
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
    pub fn sign(
        method: SolanaMethod,
        value: serde_json::Value,
    ) -> anyhow::Result<SolanaSignatureResponse> {
        let kp = Keypair::from_bytes(&KEYPAIR).map_err(|_| Error::KeyPairFailure)?;
        info!("PK of signer: {}", kp.pubkey());
        let decoded: Vec<u8> = match method {
            SolanaMethod::SignTransaction => {
                let req = serde_json::from_value::<WalletConnectTransaction>(value)?;
                BASE64_STANDARD.decode(req.transaction)?
            }
            SolanaMethod::SignMessage => {
                let req = serde_json::from_value::<SignMessageRequest>(value)?;
                bs58::decode(req.message).into_vec()?
            }
            SolanaMethod::Other(m) => return Err(Error::InvalidMethod(m).into()),
        };
        let sig = kp.sign_message(&decoded);
        // let mut tx = bincode::deserialize::<Transaction>(decoded.as_ref())?;
        // let positions = tx.get_signing_keypair_positions(&[kp.pubkey()])?;
        // if positions.is_empty() {
        //    return Err(anyhow::format_err!("nothing to sign"));
        //}
        // tx.try_partial_sign(&[&kp], tx.get_recent_blockhash().clone())?;
        //// tx.try_sign(&[&kp], tx.get_recent_blockhash().clone())?;
        // let sig = tx.get_signature();
        //let signature = bs58::encode(sig).into_string();
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
