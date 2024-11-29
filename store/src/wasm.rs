use {
    gloo_storage::{LocalStorage, Storage},
    serde::{Deserialize, Serialize},
};

#[derive(Clone)]
pub struct KvStorage {}

impl KvStorage {
    pub fn new() -> Self {
        KvStorage {}
    }

    pub fn get<T>(&self, key: impl AsRef<str>) -> crate::Result<Option<T>>
    where
        T: for<'de> Deserialize<'de> + Serialize,
    {
        match LocalStorage::get::<T>(key) {
            Err(e) => Ok(None),
            Ok(r) => Ok(Some(r)),
        }
    }

    #[tracing::instrument(level = "debug", skip(self, value), fields(key = %key.as_ref()))]
    pub fn set<T>(&self, key: impl AsRef<str>, value: T) -> crate::Result<()>
    where
        T: for<'de> Deserialize<'de> + Serialize,
    {
        Ok(LocalStorage::set(key, value)?)
    }

    #[tracing::instrument(level = "debug", skip(self), fields(key = %key.as_ref()))]
    pub fn delete(&self, key: impl AsRef<str>) -> crate::Result<()> {
        LocalStorage::delete(key);
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub fn clear(&self) {
        LocalStorage::clear();
    }

    pub fn length(&self) -> u32 {
        LocalStorage::length()
    }
}
