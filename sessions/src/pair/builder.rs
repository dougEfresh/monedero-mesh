use crate::actors::Actors;
use crate::domain::ProjectId;
use crate::{Cipher, KvStorage, PairingManager};
use std::sync::Arc;
use monedero_relay::{ConnectionOptions, SerializedAuthToken};

pub struct WalletConnectBuilder {
    connect_opts: Option<ConnectionOptions>,
    #[cfg(not(feature = "mock"))]
    auth: SerializedAuthToken,
    #[cfg(not(feature = "mock"))]
    project_id: ProjectId,
    store: Option<KvStorage>,
}

impl WalletConnectBuilder {
    ///
    ///
    pub fn new(project_id: ProjectId, auth: SerializedAuthToken) -> Self {
        Self {
            connect_opts: None,
            #[cfg(not(feature = "mock"))]
            auth,
            #[cfg(not(feature = "mock"))]
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
        #[cfg(not(feature = "mock"))]
        let opts: ConnectionOptions = match self.connect_opts {
            Some(ref opts) => opts.clone(),
            None => ConnectionOptions::new(self.project_id.clone(), self.auth.clone()),
        };

        #[cfg(feature = "mock")]
        let opts: ConnectionOptions = self
            .connect_opts
            .clone()
            .ok_or(crate::Error::InvalidateConnectionOpts)?;

        #[cfg(not(feature = "mock"))]
        let store = match self.store.as_ref() {
            Some(s) => s.clone(),
            None => KvStorage::file(None)?,
        };
        #[cfg(feature = "mock")]
        let store = KvStorage::mem();

        let store = Arc::new(store);
        let cipher = Cipher::new(store, None)?;
        PairingManager::init(opts, cipher).await
    }
}
