// ===============================================================sus=========================================
// https://specs.walletconnect.com/2.0/specs/clients/sign/namespaces#
// rejecting-a-session-response
// - validates namespaces match at least all requiredNamespaces
// ========================================================================================================

use {
    serde::{Deserialize, Serialize},
    std::{
        collections::{BTreeMap, BTreeSet},
        fmt::{Debug, Display, Formatter},
        ops::{Deref, DerefMut},
    },
};

mod account;
mod chain_id;
mod error;
mod event;
mod method;
mod name;

pub use {
    crate::{account::*, chain_id::*, event::*, method::*, name::NamespaceName},
    alloy_chains::Chain as AlloyChain,
    error::Error,
};

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Namespaces(pub BTreeMap<NamespaceName, Namespace>);

impl Namespaces {
    pub fn namespaces(&self) -> String {
        self.keys()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl Default for Namespaces {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl Debug for Namespaces {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.namespaces())
    }
}

impl Display for Namespaces {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.namespaces())
    }
}

impl Deref for Namespaces {
    type Target = BTreeMap<NamespaceName, Namespace>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Namespaces {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
// const EIP_SUPPORTED_EVENTS: &[&str] = &["chainChanged", "accountsChanged"]

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Namespace {
    #[serde(skip_serializing_if = "Accounts::is_empty", default)]
    pub accounts: Accounts,
    pub chains: Chains,
    pub methods: Methods,
    pub events: Events,
    //#[serde(skip_serializing_if = "Option::is_none")]
    //#[serde(default)]
    // pub extensions: Option<Vec<Self>>,
}

impl Namespaces {
    pub fn chains(&self) -> Chains {
        let mut chains = BTreeSet::new();
        for ns in self.deref().values().cloned() {
            for c in &ns.chains {
                chains.insert(c.clone());
            }
        }
        Chains(chains)
    }
}

impl<'a, I> From<I> for Namespaces
where
    I: IntoIterator<Item = &'a ChainId>,
{
    fn from(value: I) -> Self {
        let mut namespace_chains: BTreeMap<NamespaceName, BTreeSet<ChainId>> = BTreeMap::new();

        // Group ChainIds by their corresponding NamespaceName
        for chain_id in value {
            let namespace_name = NamespaceName::from(chain_id);
            namespace_chains
                .entry(namespace_name)
                .or_default()
                .insert(chain_id.clone());
        }

        // Create Namespace for each NamespaceName
        let namespaces: BTreeMap<NamespaceName, Namespace> = namespace_chains
            .into_iter()
            .map(|(namespace_name, chains)| {
                let methods = Methods::from(&namespace_name);
                let events = Events::from(&namespace_name);
                let accounts = Accounts(BTreeSet::new());
                (namespace_name, Namespace {
                    accounts,
                    chains: Chains(chains),
                    methods,
                    events,
                })
            })
            .collect::<BTreeMap<_, _>>();

        Self(namespaces)
    }
}

impl Namespaces {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod tests {
    use {super::*, serde_json::json};

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_into_namespace() -> anyhow::Result<()> {
        let eth = ChainId::EIP155(alloy_chains::Chain::mainnet());
        let base = ChainId::EIP155(alloy_chains::Chain::base_mainnet());
        let chains = [ChainId::Solana(ChainType::Main), eth.clone(), base.clone()];
        let namespaces: Namespaces = chains.iter().into();
        assert!(!namespaces.is_empty());
        assert_eq!(2, namespaces.len());
        let eip_ns = namespaces
            .get(&NamespaceName::EIP155)
            .ok_or(Error::NamespaceNotFound)?;
        let _ = namespaces
            .get(&NamespaceName::Solana)
            .ok_or(Error::NamespaceNotFound)?;

        assert!(eip_ns.events.is_empty());
        assert!(!eip_ns.methods.is_empty());
        assert_eq!(eip_ns.chains.len(), 2);
        let mut iter = eip_ns.chains.iter();
        assert_eq!(*iter.next().unwrap(), eth);
        assert_eq!(*iter.next().unwrap(), base);

        let eip_default_methods: BTreeSet<Method> = BTreeSet::from([
            Method::EIP155(EipMethod::PersonalSign),
            Method::EIP155(EipMethod::SendTransaction),
            Method::EIP155(EipMethod::SignTransaction),
            Method::EIP155(EipMethod::SignTypedDataV4),
            Method::EIP155(EipMethod::SignTypedData),
            Method::EIP155(EipMethod::Sign),
        ]);
        assert_eq!(eip_default_methods, eip_ns.methods.0);
        let expected_json = json!(
          {
            "eip155": {
            "chains": [
              "eip155:1",
              "eip155:8453"
            ],
            "methods": [
              "eth_sendTransaction",
              "eth_sign",
              "eth_signTransaction",
              "eth_signTypedData",
              "eth_signTypedData_v4",
              "personal_sign"
            ],
            "events": []
          },
            "solana": {
            "chains": [
              "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp"
            ],
            "methods": [
              "solana_signMessage",
              "solana_signTransaction"
            ],
            "events": []
          }
          }
        );
        // eprintln!("{}", serde_json::to_string_pretty(&namespaces)?)
        let result = serde_json::to_value(&namespaces)?;
        eprintln!("result: {:?}", result);
        assert_eq!(expected_json, result);
        Ok(())
    }
    #[test]
    #[allow(
        clippy::unwrap_used,
        clippy::too_many_lines,
        clippy::useless_vec,
        clippy::needless_collect
    )]
    fn test_deserailize() -> anyhow::Result<()> {
        let settled = json!({
          "eip155": {
            "accounts": [
              "eip155:17000:0xac56ad762E1F5335cF9e1B0F5ab78a75a93f291A",
              "eip155:11155111:0xac56ad762E1F5335cF9e1B0F5ab78a75a93f291A",
              "eip155:5:0xac56ad762E1F5335cF9e1B0F5ab78a75a93f291A"
            ],
            "methods": [
              "personal_sign",
              "eth_sign",
              "eth_signTypedData",
              "eth_signTypedData_v3",
              "eth_signTypedData_v4",
              "eth_sendTransaction",
              "eth_sendRawTransaction",
              "eth_signTransaction",
              "eth_chainId",
              "wallet_addEthereumChain",
              "wallet_switchEthereumChain",
              "eth_sendTransactionExt",
              "personal_signExt",
              "eth_signTypedDataExt"
            ],
            "events": [
              "chainChanged",
              "accountsChanged",
              "disconnect",
              "connect",
              "message"
            ],
            "chains": [
              "eip155:17000",
              "eip155:11155111",
              "eip155:5"
            ]
          },
          "solana": {
            "accounts": [
              "solana:8E9rvCKLFQia2Y35HXjjpWzj8weVo44K:Ch17QhvaWPuT6YfT7UL4sNwrbyeNH7qrKCbDSR4AWf8K"
            ],
            "methods": [
              "solana_signMessage",
              "solana_signTransaction"
            ],
            "events": [],
            "chains": [
              "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1"
            ]
          },
          "near": {
            "accounts": [
              "near:testnet:adafe9da7cd9ba340ebaabae606d9d0632b0dac5c624c573b9acf1a7f1fd6a78"
            ],
            "methods": [
              "near_signIn",
              "near_signOut",
              "near_getAccounts",
              "near_signTransaction",
              "near_signAndSendTransaction",
              "near_signTransactions",
              "near_signAndSendTransactions",
              "near_verifyOwner"
            ],
            "events": [
              "chainChanged",
              "accountsChanged"
            ],
            "chains": [
              "near:testnet"
            ]
          },
          "tezos": {
            "accounts": [
              "tezos:testnet:adafe9da7cd9ba340ebaabae606d9d0632b0dac5c624c573b9acf1a7f1fd6a78"
            ],
            "methods": [
              "tezos_getAccounts",
              "tezos_signTransaction",
            ],
            "events": [
              "chainChanged",
              "accountsChanged"
            ],
            "chains": [
              "tezos:testnet"
            ]
          }
        }
        );
        let namespaces: Namespaces = serde_json::from_value(settled)?;
        // sanity
        assert_eq!(4, namespaces.len());
        assert!(namespaces.contains_key(&NamespaceName::Solana));
        assert!(namespaces.contains_key(&NamespaceName::EIP155));
        assert!(namespaces.contains_key(&NamespaceName::Other(String::from("near"))));
        assert!(namespaces.contains_key(&NamespaceName::Other(String::from("tezos"))));

        let eip_ns = namespaces.get(&NamespaceName::EIP155).unwrap();
        let sol_ns = namespaces.get(&NamespaceName::Solana).unwrap();
        assert_eq!(3, eip_ns.accounts.len());
        assert_eq!(3, eip_ns.chains.len());
        assert_eq!(1, sol_ns.accounts.len());
        assert_eq!(1, sol_ns.chains.len());
        let accounts: Vec<Account> = eip_ns.accounts.iter().cloned().collect();
        let holesky = alloy_chains::Chain::holesky();
        let found = accounts
            .into_iter()
            .find(|a| a.chain == ChainId::EIP155(holesky));
        assert!(found.is_some());
        let accounts: Vec<Account> = sol_ns.accounts.iter().cloned().collect();
        let found = accounts
            .into_iter()
            .find(|a| a.chain == ChainId::Solana(ChainType::Dev));
        assert!(found.is_some());
        Ok(())
    }
}
