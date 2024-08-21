use crate::storage::Error::SegmentErr;
use crate::storage::Result;
use kvx::{Key, KeyValueStore, Namespace, ReadStore, Segment, WriteStore};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;
use url::Url;

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
    pub fn file(location: Option<String>) -> Result<Self> {
        let location: std::path::PathBuf = match location {
            Some(l) => std::path::PathBuf::from(l),
            None => {
                let app_name = env!("CARGO_PKG_NAME");
                let app = microxdg::XdgApp::new(app_name)?;
                app.app_cache()?
            }
        };
        debug!("using storage path location {:#?}", location);
        let namespace =
            Namespace::parse("wc2").map_err(|_| crate::storage::Error::NamespaceInvalid)?;
        let store = KeyValueStore::new(
            &Url::parse(&format!("local://{}", location.display()))?,
            namespace,
        )?;
        Ok(Self {
            store: Arc::new(store),
        })
    }

    pub fn mem() -> Self {
        let namespace =
            Namespace::parse("wc2").unwrap();
        let store = KeyValueStore::new(&Url::parse("memory://").unwrap(), namespace).unwrap();
        Self {
            store: Arc::new(store),
        }
    }

    fn key_segment(&self, key: impl AsRef<str>) -> Result<Key> {
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
        let k = self.key_segment(key)?;
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
        let k = self.key_segment(key)?;
        self.store.store(&k, serde_json::to_value(value)?)?;
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self), fields(key = %key.as_ref()))]
    pub fn delete(&self, key: impl AsRef<str>) -> Result<()> {
        let k = self.key_segment(key)?;
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

    pub fn length(&self) -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use walletconnect_sdk::rpc::domain::Topic;

    pub fn test_storage_kv(store: KvStorage) -> anyhow::Result<()> {
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
        test_storage_kv(store)
    }
    #[test]
    pub fn test_storage_kv_file() -> anyhow::Result<()> {
        let topic = Topic::generate();
        let store = KvStorage::file(Some(format!("./target/kv/{topic}")))?;
        test_storage_kv(store)
    }
}
