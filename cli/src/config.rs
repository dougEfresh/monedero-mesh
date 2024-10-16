use anyhow::format_err;
use config::{Config, File, FileFormat};
use microxdg::{XdgApp, XdgError};
use monedero_namespaces::{AlloyChain, ChainId, ChainType, Chains};
use serde::Deserialize;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tracing::Instrument;

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub devnet: bool,
    chains: Option<Chains>,
    pub solana_rpc: String,
    pub etherscan_api_key: Option<String>,
    pub storage_path: Option<String>,
}

#[allow(clippy::option_if_let_else)]
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

impl AppConfig {
    pub fn new(profile: Option<String>) -> anyhow::Result<Self> {
        let x = XdgApp::new(env!("CARGO_CRATE_NAME"))?;
        match x.app_config_file("config.toml") {
            Ok(p) => {
                let cfg =
                    Config::builder().add_source(File::new(p.to_str().unwrap(), FileFormat::Toml));
                let conf: Self = cfg.build()?.try_deserialize()?;
                Ok(conf)
            }
            Err(_) => Ok(AppConfig::default()),
        }
        /*
        tracing::debug!("using config {}", cfg);
        if let Some(p) = profile {
            let x = XdgApp::new(env!("CARGO_CRATE_NAME"))?;
            let p = x.app_config_file(p)?;
            return Self::new_with_override(cfg, Some(p));
        }
        Self::new_with_override(cfg, None)
         */
    }

    fn new_with_override(cfg_default: &str, cfg_override: Option<PathBuf>) -> anyhow::Result<Self> {
        let p = expand_tilde(cfg_default);
        let mut cfg = Config::builder().add_source(File::new(&p, FileFormat::Toml));
        if let Some(profile) = cfg_override {
            cfg = cfg.add_source(
                File::new(&profile.display().to_string(), FileFormat::Toml).required(true),
            );
        }
        cfg = cfg.add_source(config::Environment::with_prefix("FIREBLOCKS").try_parsing(true));
        let conf: Self = cfg.build()?.try_deserialize()?;
        tracing::trace!("loaded config {conf:#?}");
        Ok(conf)
    }

    pub fn rpc_client(&self) -> Arc<RpcClient> {
        let custom_rpc = std::env::var("solana_8E9rvCKLFQia2Y35HXjjpWzj8weVo44K").ok();
        let rpc_url = if custom_rpc.is_some() {
            custom_rpc.unwrap()
        } else {
            self.solana_rpc.clone()
        };
        Arc::new(RpcClient::new(rpc_url))
    }

    pub fn storage(&self) -> anyhow::Result<PathBuf> {
        match &self.storage_path {
            None => {
                let x = XdgApp::new(env!("CARGO_CRATE_NAME"))?;
                Ok(x.app_cache()?)
            }
            Some(path) => Ok(PathBuf::from_str(path)?),
        }
    }
}

pub const DEFAULT_CHAINS: [ChainId; 4] = [
    ChainId::Solana(ChainType::Test),
    ChainId::EIP155(AlloyChain::sepolia()),
    ChainId::EIP155(AlloyChain::base_sepolia()),
    ChainId::EIP155(AlloyChain::optimism_sepolia()),
];

impl AppConfig {
    pub fn chains(&self) -> Chains {
        match &self.chains {
            None => {
                DEFAULT_CHAINS.into()
                //DEFAULT_CHAINS.iter().cloned().into()
            }
            Some(chains) => chains.clone(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let chains = [
            ChainId::Solana(ChainType::Test),
            ChainId::EIP155(AlloyChain::sepolia()),
            ChainId::EIP155(AlloyChain::base_sepolia()),
            ChainId::EIP155(AlloyChain::optimism_sepolia()),
        ];
        Self {
            devnet: true,
            chains: Some(chains.into()),
            solana_rpc: "https://api.devnet.solana.com".to_string(),
            etherscan_api_key: None,
            storage_path: None,
        }
    }
}
