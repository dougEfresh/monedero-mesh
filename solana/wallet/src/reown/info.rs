use {
    super::account::ReownAccountInfo,
    crate::WALLET_FEATURES,
    std::fmt::Debug,
    wallet_standard::WalletInfo,
};

#[derive(Clone)]
pub struct ReownInfo {
    accounts: Vec<ReownAccountInfo>,
}

impl ReownInfo {
    pub fn new_session(accounts: Vec<ReownAccountInfo>) -> Self {
        Self { accounts }
    }

    pub fn account(&self) -> Option<ReownAccountInfo> {
        self.accounts.first().cloned()
    }
}

impl Debug for ReownInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.accounts.first() {
            Some(a) => write!(f, "{a}"),
            None => write!(f, "no-accounts"),
        }
    }
}

const VERSION: &str = env!["CARGO_PKG_VERSION"];

impl WalletInfo for ReownInfo {
    type Account = ReownAccountInfo;

    fn version(&self) -> String {
        VERSION.to_owned()
    }

    fn name(&self) -> String {
        "Solana Reown Wallet".into()
    }

    fn icon(&self) -> String {
        String::new()
    }

    fn chains(&self) -> Vec<String> {
        vec!["solana".into()]
    }

    fn features(&self) -> Vec<String> {
        WALLET_FEATURES.map(Into::into).to_vec()
    }

    fn accounts(&self) -> Vec<Self::Account> {
        self.accounts.clone()
    }
}
