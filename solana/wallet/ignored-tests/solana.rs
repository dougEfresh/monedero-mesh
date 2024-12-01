use {
    anyhow::format_err,
    assert_matches::assert_matches,
    async_trait::async_trait,
    base64::{prelude::BASE64_STANDARD, Engine},
    monedero_mesh::{
        crypto::CipherError,
        rpc::{
            Metadata,
            ResponseParamsError,
            ResponseParamsSuccess,
            RpcResponsePayload,
            SessionProposeRequest,
            SessionProposeResponse,
            SessionRequestRequest,
        },
        Actors,
        ClientSession,
        Dapp,
        NoopSessionHandler,
        ProjectId,
        ProposeFuture,
        RegisteredComponents,
        SdkErrors,
        Topic,
        Wallet,
        WalletConnectBuilder,
        WalletRequestResponse,
        WalletSettlementHandler,
    },
    monedero_domain::namespaces::{
        Account,
        Accounts,
        AlloyChain,
        ChainId,
        ChainType,
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
    monedero_relay::{auth_token, ConnectionCategory, ConnectionOptions, ConnectionPair},
    monedero_solana::{
        Error,
        KeyedStakeState,
        ReownSigner,
        Result,
        SolanaSession,
        SolanaSignatureResponse,
        SolanaWallet,
        StakeClient,
        StakeType,
        TokenMintClient,
        TokenSymbolDev,
        TokenTransferClient,
        WalletConnectTransaction,
    },
    serde::Deserialize,
    solana_program::native_token::LAMPORTS_PER_SOL,
    solana_rpc_client::{
        nonblocking::rpc_client::RpcClient,
        rpc_client::{RpcClientConfig, SerializableTransaction},
    },
    solana_sdk::{
        message::Message,
        pubkey::Pubkey,
        signature::{Keypair, Signature},
        signer::Signer,
        transaction::{Transaction, VersionedTransaction},
    },
    std::{
        collections::{BTreeMap, BTreeSet},
        path::{Path, PathBuf},
        str::FromStr,
        sync::{Arc, Once},
        time::Duration,
    },
    tokio::time::timeout,
    tracing::{error, info},
    tracing_subscriber::{fmt::format::FmtSpan, EnvFilter},
};

#[allow(dead_code)]
static INIT: Once = Once::new();

fn explorer(sig: &Signature) {
    info!("https://solscan.io/tx/{sig}?cluster=custom&customUrl=https://soldev.dougchimento.com");
}

pub(crate) async fn yield_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

#[derive(Clone)]
struct MockWallet {
    rpc_client: Arc<RpcClient>,
}

const SUPPORTED_ACCOUNT: &str = "215r9xfTFVYcE9g3fAUGowauM84egyUvFCbSo3LKNaep";

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

const KEYPAIR: [u8; 64] = [
    186, 128, 232, 61, 254, 246, 197, 13, 125, 103, 212, 83, 16, 121, 144, 20, 93, 161, 35, 128,
    89, 135, 157, 200, 81, 159, 83, 204, 204, 130, 28, 42, 14, 225, 43, 2, 44, 16, 255, 214, 161,
    18, 184, 164, 253, 126, 16, 187, 134, 176, 75, 35, 179, 194, 181, 150, 67, 236, 131, 49, 45,
    155, 45, 253,
];

impl MockWallet {
    pub async fn sign(&self, value: serde_json::Value) -> Result<SolanaSignatureResponse> {
        let kp = Keypair::from_bytes(&KEYPAIR).map_err(|_| Error::KeyPairFailure)?;
        info!("PK of signer: {}", kp.pubkey());
        let req = serde_json::from_value::<WalletConnectTransaction>(value)?;
        let decoded = BASE64_STANDARD.decode(req.transaction)?;
        let mut tx = bincode::deserialize::<Transaction>(decoded.as_ref())?;
        let positions = tx.get_signing_keypair_positions(&[kp.pubkey()])?;
        if positions.is_empty() {
            return Err(Error::NothingToSign);
        }
        tx.try_partial_sign(&[&kp], tx.get_recent_blockhash().clone())?;
        // tx.try_sign(&[&kp], tx.get_recent_blockhash().clone())?;
        let sig = tx.get_signature();
        let signature = solana_sdk::bs58::encode(sig).into_string();
        info!("returning sig: {signature}");
        Ok(SolanaSignatureResponse { signature })
    }
}

#[async_trait]
impl monedero_mesh::SessionHandler for MockWallet {
    async fn request(&self, request: SessionRequestRequest) -> WalletRequestResponse {
        match request.request.method {
            Method::Solana(SolanaMethod::SignTransaction) => {
                match self.sign(request.request.params).await {
                    Err(e) => {
                        tracing::warn!("failed sig: {e}");
                        WalletRequestResponse::Error(SdkErrors::UserRejected)
                    }
                    Ok(sig) => WalletRequestResponse::Success(serde_json::to_value(&sig).unwrap()),
                }
            }
            _ => WalletRequestResponse::Error(SdkErrors::InvalidMethod),
        }
    }
}

pub(crate) async fn init_test_components() -> anyhow::Result<(Dapp, Wallet, MockWallet)> {
    init_tracing();
    let shared_id = Topic::generate();
    let p = ProjectId::from("987f2292c12194ae69ddb6c52ceb1d62");
    let auth = auth_token("https://github.com/dougEfresh");
    let dapp_id = ConnectionPair(shared_id.clone(), ConnectionCategory::Dapp);
    let wallet_id = ConnectionPair(shared_id.clone(), ConnectionCategory::Wallet);
    let dapp_opts = ConnectionOptions::new(p.clone(), auth.clone(), dapp_id);
    let wallet_opts = ConnectionOptions::new(p.clone(), auth.clone(), wallet_id);
    let dapp_manager = WalletConnectBuilder::new(p.clone(), auth.clone())
        .connect_opts(dapp_opts)
        .build()
        .await?;
    let wallet_manager = WalletConnectBuilder::new(p, auth)
        .connect_opts(wallet_opts)
        .build()
        .await?;
    let md = Metadata {
        name: "mock-dapp".to_string(),
        ..Default::default()
    };
    let dapp = Dapp::new(dapp_manager, md).await?;
    dotenvy::dotenv()?;
    // let url = std::env::var(ChainId::Solana(ChainType::Test).to_string())
    //.ok()
    //.unwrap_or(String::from("https://api.devnet.solana.com"));
    let url = std::env::var(ChainId::Solana(ChainType::Dev).to_string())
        .ok()
        .unwrap_or(String::from("https://soldev.dougchimento.com"));
    info!("using url {url}");
    let rpc_client = Arc::new(RpcClient::new(url));
    let mock_wallet = MockWallet { rpc_client };
    let wallet = Wallet::new(wallet_manager, mock_wallet.clone()).await?;
    Ok((dapp, wallet, mock_wallet))
}

pub(crate) fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_target(true)
            .with_level(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    });
}

async fn await_wallet_pair(rx: ProposeFuture) {
    match timeout(Duration::from_secs(5), rx).await {
        Ok(s) => match s {
            Ok(_) => {
                info!("wallet got client session")
            }
            Err(e) => error!("wallet got error session: {e}"),
        },
        Err(e) => error!("timeout for wallet to recv client session from wallet: {e}"),
    }
}

async fn pair_dapp_wallet() -> anyhow::Result<(SolanaWallet, MockWallet)> {
    let (dapp, wallet, mock_wallet) = init_test_components().await?;
    let (pairing, rx, _) = dapp
        .propose(NoopSessionHandler, &[ChainId::Solana(ChainType::Dev)])
        .await?;
    info!("got pairing topic {pairing}");
    let (_, wallet_rx) = wallet
        .pair(pairing.to_string(), mock_wallet.clone())
        .await?;
    tokio::spawn(async move { await_wallet_pair(wallet_rx).await });
    let session = timeout(Duration::from_secs(5), rx).await??;
    // let wallet get their ClientSession
    yield_ms(1000).await;
    let sol_session = SolanaSession::try_from(&session)?;
    let w = SolanaWallet::init(
        sol_session,
        mock_wallet.rpc_client.clone(),
        PathBuf::from(Path::new("/tmp")),
        1000000,
        Some("testing"),
    )
    .await?;
    Ok((w, mock_wallet))
}
#[tokio::test(flavor = "multi_thread", worker_threads = 10)]

async fn test_solana_session() -> anyhow::Result<()> {
    let (wallet, _) = pair_dapp_wallet().await?;
    info!("settlement complete");
    let to = Pubkey::from_str("E4SfgGV2v9GLYsEkCQhrrnFbBcYmAiUZZbJ7swKGzZHJ")?;
    let amount = 1;
    let balance = wallet.balance().await?;
    info!("balance in lamports {balance}");
    let sig = wallet.transfer(&to, amount).await?;
    info!("got sig {sig}");
    Ok(())
}


#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_solana_tokens() -> anyhow::Result<()> {
    let (wallet, _) = pair_dapp_wallet().await?;
    let token_account_client = wallet.token_accounts_client();
    let accounts = token_account_client.accounts().await?.accounts;
    let usdc = accounts
        .iter()
        .find(|a| a.metadata.symbol == "USDC")
        .expect("No USDC");
    let token_client = wallet.token_transfer_client(usdc);
    let balance = token_client.balance().await?;
    let to = Pubkey::from_str("E4SfgGV2v9GLYsEkCQhrrnFbBcYmAiUZZbJ7swKGzZHJ")?;
    info!(
        "balance {} on token account {}  (wallet:{})",
        balance,
        token_client.account(),
        wallet.pk()
    );
    info!("sending to {to}");
    let sig = token_client.transfer(&to, 1).await?;
    info!("got signature {sig}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_solana_mint() -> anyhow::Result<()> {
    let (wallet, mock_wallet) = pair_dapp_wallet().await?;
    let to = Pubkey::from_str("E4SfgGV2v9GLYsEkCQhrrnFbBcYmAiUZZbJ7swKGzZHJ")?;
    let mint = wallet.token_mint_client();
    let (token_address, sig) = mint.create_mint(6, None).await?;
    info!("created mint {token_address} signature: {sig}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_solana_wrap_sol() -> anyhow::Result<()> {
    let (wallet, mock_wallet) = pair_dapp_wallet().await?;
    let token_client = wallet.token_wrapped_client();
    info!("wrapped account {}", token_client.account());
    let wrapped = token_client.wrap(5000, false).await?;
    let balance = token_client.balance().await?;
    info!("immutable wrapped {wrapped} balance: {balance}");
    let wrapped = token_client.wrap(5000, true).await?;
    let balance = token_client.balance().await?;
    info!("mut wrapped {wrapped} balance: {balance}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_solana_mint_new_tokens() -> anyhow::Result<()> {
    // this came from  test_solana_mint above
    let token_address = Pubkey::from_str("8m3uKEn4fMPNVr7nv6RmQYktT4zRqEZzhuZDpG8hQZT4")?;
    let (wallet, _) = pair_dapp_wallet().await?;
    let accounts = wallet.token_accounts_client().accounts().await?.accounts;
    let my_token_account = accounts
        .iter()
        .find(|a| a.address == *wallet.pk())
        .expect("failed to find my token!");
    let to = Pubkey::from_str("E4SfgGV2v9GLYsEkCQhrrnFbBcYmAiUZZbJ7swKGzZHJ")?;
    let token_client = wallet.token_transfer_client(my_token_account);
    let sig = token_client.mint_to(wallet.pk(), 100_000_000).await?;
    info!("signature for minting to me {sig}");
    let sig = token_client.transfer(&to, 1000000).await?;
    info!("signature for sending to {to} {sig}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_solana_stake_accounts() -> anyhow::Result<()> {
    let (wallet, _) = pair_dapp_wallet().await?;
    info!("settlement complete {}", wallet.pk());
    let staker = wallet.stake_client();
    let validators = staker.validators().await?;
    info!("there are {} validators", validators.len());
    let accounts = staker.accounts().await?;
    info!("you have {} staked accounts", accounts.len());
    for a in &accounts {
        info!("{a}")
    }
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    info!("using seed {seed}");
    let (account, sig) = staker.create(seed, LAMPORTS_PER_SOL * 2).await?;
    info!("create new account {account} sig: {sig}");
    explorer(&sig);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_solana_stake_withdrawal() -> anyhow::Result<()> {
    let (wallet, _) = pair_dapp_wallet().await?;
    info!("settlement complete {}", wallet.pk());
    let staker = wallet.stake_client();
    let validators = staker.validators().await?;
    info!("there are {} validators", validators.len());
    let accounts = staker.accounts_undelegated().await?;
    if accounts.is_empty() {
        info!("no accounts to withdrawal");
        return Ok(());
    }
    let unstake = &accounts[0];
    info!("withdrawl from {}", unstake);
    let sig = staker.withdraw(unstake).await?;
    explorer(&sig);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_solana_stake_delegate() -> anyhow::Result<()> {
    let (wallet, _) = pair_dapp_wallet().await?;
    let staker = wallet.stake_client();
    let validators = staker.validators().await?;
    info!("there are {} validators", validators.len());
    let accounts = staker.accounts_undelegated().await?;
    if accounts.is_empty() {
        info!("no accounts to delegate");
        return Ok(());
    }
    for a in &accounts {
        info!("{a}")
    }
    let a = &accounts[0];
    let v = Pubkey::from_str(&validators[0].vote_pubkey)?;
    let sig = staker.delegate(a, &v).await?;
    explorer(&sig);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_solana_stake_create_delegate() -> anyhow::Result<()> {
    let (wallet, _) = pair_dapp_wallet().await?;
    let staker = wallet.stake_client();
    let validators = staker.validators().await?;
    info!("there are {} validators", validators.len());
    let v = Pubkey::from_str(&validators[0].vote_pubkey)?;
    let (stake_account, sig) = staker
        .create_delegate((2.1 * LAMPORTS_PER_SOL as f64) as u64, &v)
        .await?;
    info!("created stake account {stake_account}");
    explorer(&sig);
    Ok(())
}
