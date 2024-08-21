use crate::domain::ProjectId;
use crate::relay::ConnectionOptions;
use crate::rpc::Metadata;
use crate::{KvStorage, PairingManager, RELAY_ADDRESS};
use walletconnect_sdk::rpc::auth::SerializedAuthToken;

pub struct WalletConnectBuilder {
    connect_opts: Option<ConnectionOptions>,
    metadata: Option<Metadata>,
    auth: SerializedAuthToken,
    name: Option<String>,
    description: String,
    icons: Vec<String>,
    project_id: ProjectId,
    store: Option<KvStorage>,
    mock: bool
}

impl WalletConnectBuilder {
    ///
    ///
    pub fn new(project_id: ProjectId, auth: SerializedAuthToken) -> Self {
        Self {
            connect_opts: None,
            metadata: None,
            description: env!["CARGO_PKG_NAME"].to_string(),
            auth,
            project_id,
            icons: vec!["https://avatars.githubusercontent.com/u/976425?v=4".to_owned()],
            name: None,
            store: None,
            mock: false
        }
    }

    pub fn metadata(mut self, md: Metadata) -> Self {
        self.metadata = Some(md);
        self
    }

    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    pub fn connect_opts(mut self, opts: ConnectionOptions) -> Self {
        self.connect_opts = Some(opts);
        self
    }

    pub fn mock(mut self, mock: bool) -> Self {
        self.mock = mock;
        self
    }

    pub fn store(mut self, store: KvStorage) -> Self {
        self.store = Some(store);
        self
    }

    pub async fn build(&self) -> crate::Result<PairingManager> {
        let md: Metadata = match self.metadata {
            Some(ref metadata) => metadata.clone(),
            None => Metadata {
                name: String::from(
                    self.name
                        .as_ref()
                        .unwrap_or(&String::from("walletconnect for rust")),
                ),
                description: self.description.to_string(),
                url: "https://crate.rs/walletconnect-sessions".to_string(),
                icons: Vec::clone(&self.icons),
                verify_url: None,
                redirect: None,
            },
        };

        let opts: ConnectionOptions = match self.connect_opts {
            Some(ref opts) => opts.clone(),
            None => ConnectionOptions::new(self.project_id.clone(), self.auth.clone()).mock(self.mock)
        };

        let store = match self.store.as_ref() {
            Some(s) => s.clone(),
            None => KvStorage::file(None)?,
        };
        PairingManager::init(md, opts, store).await
    }
}
