use {
    crate::PairingManager,
    monedero_cipher::Cipher,
    monedero_domain::ProjectId,
    monedero_relay::{ConnectionOptions, SerializedAuthToken},
    monedero_store::KvStorage,
    std::sync::Arc,
};

pub struct WalletConnectBuilder {
    connect_opts: Option<ConnectionOptions>,
    auth: SerializedAuthToken,
    project_id: ProjectId,
    store: Option<KvStorage>,
}

impl WalletConnectBuilder {
    ///
    pub fn new(project_id: ProjectId, auth: SerializedAuthToken) -> Self {
        Self {
            connect_opts: None,
            auth,
            project_id,
            store: None,
        }
    }

    pub fn connect_opts(mut self, opts: ConnectionOptions) -> Self {
        self.connect_opts = Some(opts);
        self
    }

    pub fn store(mut self, store: KvStorage) -> Self {
        self.store = Some(store);
        self
    }

    pub async fn build(&self) -> crate::Result<PairingManager> {
        let opts: ConnectionOptions = match self.connect_opts {
            Some(ref opts) => opts.clone(),
            None => ConnectionOptions::new(self.project_id.clone(), self.auth.clone()),
        };

        #[cfg(not(target_arch = "wasm32"))]
        let store = match self.store.as_ref() {
            Some(s) => s.clone(),
            None => KvStorage::file(None)?,
        };

        #[cfg(target_arch = "wasm32")]
        let store = KvStorage::new();

        let store = Arc::new(store);
        let cipher = Cipher::new(store, None)?;
        PairingManager::init(opts, cipher).await
    }
}
