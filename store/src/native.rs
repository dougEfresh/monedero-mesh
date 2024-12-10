use {
    crate::{Error::SegmentErr, Result},
    kvx::{Key, KeyValueStore, Namespace, ReadStore, Segment, WriteStore},
    serde::{Deserialize, Serialize},
    sha2::Digest,
    std::{
        path::{Path, PathBuf},
        sync::Arc,
    },
    tracing::{debug, info},
    url::Url,
};

#[derive(Clone)]
pub struct KvStorage {
    store: Arc<KeyValueStore>,
}

impl Default for KvStorage {
    fn default() -> Self {
        Self::mem()
    }
}

impl KvStorage {
    pub fn path(location: &Path, ns: &str) -> Result<Self> {
        info!("using storage path location {}", location.display());
        let namespace = Namespace::parse(ns).map_err(|_| crate::Error::NamespaceInvalid)?;
        let store = KeyValueStore::new(
            &Url::parse(&format!("local://{}", location.display()))?,
            namespace,
        )?;
        Ok(Self {
            store: Arc::new(store),
        })
    }

    pub fn file(location: Option<String>) -> Result<Self> {
        let location: PathBuf = if let Some(l) = location {
            std::path::PathBuf::from(l)
        } else {
            let app_name = env!("CARGO_PKG_NAME");
            let app = microxdg::XdgApp::new(app_name).map_err(|e| {
                crate::Error::StorageInit(format!(
                    "failed to find local storage for {app_name} {e}"
                ))
            })?;
            app.app_cache().map_err(|e| {
                crate::Error::StorageInit(format!(
                    "failed to create cache storage for {app_name} {e}"
                ))
            })?
        };
        Self::path(&location, "wc2")
    }

    pub fn new() -> Self {
        Self::mem()
    }

    /// # Panics
    ///
    /// Possible but very unlikely panic
    pub fn mem() -> Self {
        // create random namespace to avoid collision
        let r = rand::random::<u64>();
        // Hash it
        let mut hasher = sha2::Sha256::new();
        hasher.update(r.to_le_bytes());
        let result = hasher.finalize();
        let id = hex::encode(&result[..16]);
        let namespace = Namespace::parse(&id).unwrap();
        let store = KeyValueStore::new(&Url::parse("memory://").unwrap(), namespace).unwrap();
        Self {
            store: Arc::new(store),
        }
    }

    pub(super) fn key_segment(key: impl AsRef<str>) -> Result<Key> {
        let seg =
            Segment::parse(key.as_ref()).map_err(|_| SegmentErr(String::from(key.as_ref())))?;
        Ok(Key::new_global(seg))
    }
}

impl KvStorage {
    #[tracing::instrument(level = "debug", skip(self), fields(key = %key.as_ref()))]
    pub fn get<T>(&self, key: impl AsRef<str>) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de> + Serialize,
    {
        let k = Self::key_segment(key)?;
        if !self.store.has(&k)? {
            return Ok(None);
        }
        match self.store.get(&k)? {
            Some(v) => Ok(Some(serde_json::from_value(v)?)),
            None => Ok(None),
        }
    }

    #[tracing::instrument(level = "debug", skip(self, value), fields(key = %key.as_ref()))]
    pub fn set<T>(&self, key: impl AsRef<str>, value: T) -> Result<()>
    where
        T: for<'de> Deserialize<'de> + Serialize,
    {
        let k = Self::key_segment(key)?;
        self.store.store(&k, serde_json::to_value(value)?)?;
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self), fields(key = %key.as_ref()))]
    pub fn delete(&self, key: impl AsRef<str>) -> Result<()> {
        let k = Self::key_segment(key)?;
        if !self.store.has(&k)? {
            return Ok(());
        }
        self.store.delete(&k)?;
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub fn clear(&self) {
        if let Err(e) = self.store.clear() {
            debug!("failed to clear storage {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, monedero_relay::Topic};

    pub fn test_storage_kv(store: &KvStorage) -> anyhow::Result<()> {
        let result = store.get::<String>("mine")?;
        assert!(result.is_none());
        let store_me: String = String::from("something");
        store.set("mine", store_me)?;
        let result = store.get::<String>("mine")?;
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result, "something");
        store.delete("mine")?;
        assert!(store.get::<String>("mine")?.is_none());

        store.set("mine", Topic::generate())?;
        store.clear();
        assert!(store.get::<String>("mine")?.is_none());
        Ok(())
    }

    #[test]
    pub fn test_storage_kv_mem() -> anyhow::Result<()> {
        let store = KvStorage::mem();
        test_storage_kv(&store)
    }

    #[test]
    pub fn test_storage_kv_file() -> anyhow::Result<()> {
        let topic = Topic::generate();
        let target_dir = env!("CARGO_TARGET_DIR");
        let store = KvStorage::file(Some(format!("{target_dir}/kv/{topic}")))?;
        test_storage_kv(&store)
    }
}
