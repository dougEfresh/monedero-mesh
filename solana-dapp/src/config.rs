use anyhow::format_err;
use microxdg::XdgApp;
use monedero_namespaces::{AlloyChain, ChainId, ChainType, Chains};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub devnet: bool,
    pub chains: Chains,
    pub solana_rpc: String,
    pub etherscan_api_key: Option<String>,
    pub cache_path: Option<String>,
}

fn default_cache_path() -> anyhow::Result<String> {
    let x = XdgApp::new(env!("CARGO_CRATE_NAME"))?;
    Ok(String::from(
        x.app_cache()?
            .as_path()
            .as_os_str()
            .to_str()
            .ok_or(format_err!("no cache for you"))?,
    ))
}

impl Default for AppConfig {
    fn default() -> Self {
        let chains = [
            ChainId::Solana(ChainType::Test),
            ChainId::EIP155(AlloyChain::sepolia()),
            ChainId::EIP155(AlloyChain::base_sepolia()),
            ChainId::EIP155(AlloyChain::optimism_sepolia()),
        ];
        let cache_path = Some(default_cache_path().unwrap_or(String::from("/tmp")));
        Self {
            devnet: true,
            chains: chains.into(),
            solana_rpc: "https://api.devnet.solana.com".to_string(),
            etherscan_api_key: None,
            cache_path,
        }
    }
}
