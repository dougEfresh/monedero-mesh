use {
    crate::{auth_token, PairingManager, AUTH_URL},
    monedero_cipher::Cipher,
    monedero_domain::ProjectId,
    monedero_relay::{ConnectionOptions, SerializedAuthToken},
    monedero_store::KvStorage,
    std::sync::Arc,
    tracing::warn,
};

pub struct ReownBuilder {
    connect_opts: Option<ConnectionOptions>,
    auth: Option<SerializedAuthToken>,
    project_id: ProjectId,
    store: Option<KvStorage>,
}

impl ReownBuilder {
    pub fn new(project_id: ProjectId) -> Self {
        Self {
            connect_opts: None,
            auth: None,
            project_id,
            store: None,
        }
    }

    #[must_use]
    pub fn connect_opts(mut self, opts: ConnectionOptions) -> Self {
        self.connect_opts = Some(opts);
        self
    }

    #[must_use]
    pub fn store(mut self, store: KvStorage) -> Self {
        self.store = Some(store);
        self
    }

    #[must_use]
    pub fn auth(mut self, auth: SerializedAuthToken) -> Self {
        self.auth = Some(auth);
        self
    }

    pub async fn build(&self) -> crate::Result<PairingManager> {
        let auth: SerializedAuthToken = self.auth.as_ref().map_or_else(
            || {
                if self.connect_opts.is_none() {
                    warn!("using default auth URL {AUTH_URL}");
                }
                auth_token(AUTH_URL)
            },
            std::clone::Clone::clone,
        );

        let opts: ConnectionOptions = self.connect_opts.as_ref().map_or_else(
            || ConnectionOptions::new(self.project_id.clone(), auth.clone()),
            std::clone::Clone::clone,
        );

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
