use {
    config::{Config, File, FileFormat},
    microxdg::XdgApp,
    monedero_mesh::domain::namespaces::{AlloyChain, ChainId, ChainType, Chains},
    monedero_solana::RpcClient,
    serde::Deserialize,
    std::{
        fmt::Display,
        path::{Path, PathBuf},
        str::FromStr,
        sync::Arc,
    },
};

const APP_NAME: &str = env!("CARGO_BIN_NAME");
const SOLANA_RPC_DEV: &str = "https://api.devnet.solana.com";
#[allow(dead_code)]
const SOLANA_RPC_TEST: &str = "https://api.devnet.solana.com";
const SOLANA_RPC_MAIN: &str = "https://api.mainnet.solana.com";

const DEFAULT_CHAINS_DEV: [ChainId; 4] = [
    ChainId::Solana(ChainType::Dev),
    ChainId::EIP155(AlloyChain::sepolia()),
    ChainId::EIP155(AlloyChain::base_sepolia()),
    ChainId::EIP155(AlloyChain::optimism_sepolia()),
];

const DEFAULT_CHAINS_MAIN: [ChainId; 2] = [
    ChainId::Solana(ChainType::Main),
    ChainId::EIP155(AlloyChain::mainnet()),
    // ChainId::EIP155(AlloyChain::base_mainnet()),
    // ChainId::EIP155(AlloyChain::optimism_mainnet()),
];

#[derive(Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub chain_type: ChainType,
    chains: Option<Chains>,
    pub solana_rpc: Option<String>,
    #[allow(dead_code)]
    pub etherscan_api_key: Option<String>,
    pub storage_path: Option<String>,
}

impl Display for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let p = self.storage().unwrap_or_default();
        write!(
            f,
            "chain:{} rpc:{} storage:{}",
            self.chain_type,
            self.solana_rpc(),
            p.display()
        )
    }
}

#[allow(clippy::option_if_let_else, dead_code)]
fn expand_tilde(path: &str) -> String {
    if path.starts_with('~') {
        match dirs::home_dir() {
            Some(mut home) => {
                home.push(&path[2..]); // Skip the '~/' part of the string
                home.as_path().display().to_string()
            }
            None => String::from(path), // Home directory not found, return original
        }
    } else {
        String::from(path)
    }
}

fn default_config_file() -> anyhow::Result<PathBuf> {
    config_file("config.toml")
}

fn config_file(name: &str) -> anyhow::Result<PathBuf> {
    let x = XdgApp::new(APP_NAME)?;
    let mut p = x.app_config()?;
    p.push(name);
    Ok(p)
}

impl AppConfig {
    pub fn new(cfg: Option<PathBuf>, profile: Option<String>) -> anyhow::Result<Self> {
        let cfg = match cfg {
            None => default_config_file()?,
            Some(cfg) => cfg,
        };
        Self::new_with_override(&cfg, profile)
    }

    fn new_with_override(cfg_default: &Path, cfg_override: Option<String>) -> anyhow::Result<Self> {
        tracing::debug!("Loading config {}", cfg_default.display());
        let p = format!("{}", cfg_default.display());
        let mut cfg = Config::builder().add_source(File::new(&p, FileFormat::Toml).required(true));
        if let Some(profile) = cfg_override {
            let profile_loc = format!("{profile}.toml");
            let profile_loc = config_file(profile_loc.as_str())?;
            tracing::debug!("Loading profile config {}", profile_loc.display());
            cfg = cfg.add_source(
                File::new(&profile_loc.display().to_string(), FileFormat::Toml).required(true),
            );
        }
        cfg = cfg.add_source(config::Environment::with_prefix("MONEDERO").try_parsing(true));
        let conf: Self = cfg.build()?.try_deserialize()?;
        Ok(conf)
    }

    fn solana_rpc(&self) -> String {
        self.solana_rpc.as_ref().map_or_else(
            || match self.chain_type {
                ChainType::Main => String::from(SOLANA_RPC_MAIN),
                _ => String::from(SOLANA_RPC_DEV),
            },
            std::clone::Clone::clone,
        )
    }

    pub fn solana_rpc_client(&self) -> Arc<RpcClient> {
        Arc::new(RpcClient::new(&self.solana_rpc()))
    }

    pub fn storage(&self) -> anyhow::Result<PathBuf> {
        match &self.storage_path {
            None => {
                let x = XdgApp::new(APP_NAME)?;
                let mut p = x.app_cache()?;
                p.push(self.chain_type.to_string());
                Ok(p)
            }
            Some(path) => Ok(PathBuf::from_str(path)?),
        }
    }
}

impl AppConfig {
    pub fn chains(&self) -> Chains {
        self.chains.as_ref().map_or_else(
            || match self.chain_type {
                ChainType::Main => DEFAULT_CHAINS_MAIN.into(),
                ChainType::Test | ChainType::Dev => DEFAULT_CHAINS_DEV.into(),
            },
            std::clone::Clone::clone,
        )
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let chains = DEFAULT_CHAINS_DEV.clone();
        Self {
            chain_type: ChainType::Test,
            chains: Some(chains.into()),
            solana_rpc: Some(SOLANA_RPC_DEV.to_string()),
            etherscan_api_key: None,
            storage_path: None,
        }
    }
}
