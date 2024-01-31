use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

use crate::{
    errors::{DriverError, MirrorError, MirrorResult},
    store::Store,
};

/// Enum representing the mirroring policy for [`MultiStore`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Policy {
    /// Continue mirroring to other stores even if one fails
    ContinueOnFailure,
    /// Stop mirroring if any store fails
    StopOnFailure,
}

/// Struct representing a [`MultiStore`] that manages multiple stores, including
/// a primary store and mirrors.
#[derive(Clone)]
pub struct MultiStore {
    pub primary: Store,
    mirrors: HashMap<String, Vec<String>>,
    mirrors_policy: Policy,
    stores: HashMap<String, Store>,
}

impl MultiStore {
    /// Creates a new [`MultiStore`] with the provided primary store.
    ///
    /// # Example
    /// ```rust
    /// use std::{collections::HashMap, path::PathBuf};
    /// use active_storage::{drivers, multi_store::MultiStore, StoreConfig};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = drivers::disk::Config {
    ///         location: PathBuf::from("tmp").join("primary-storage"),
    ///     };
    ///     let disk_driver = StoreConfig::Disk(config).build().await.unwrap();
    ///
    ///     let inmem_driver = StoreConfig::InMem().build().await.unwrap();
    ///
    ///     let mut multi_store = MultiStore::new(disk_driver);
    ///     multi_store.add_stores(HashMap::from([("secondary", inmem_driver)]));
    /// }    
    /// ```
    #[must_use]
    pub fn new(store: Store) -> Self {
        Self {
            primary: store,
            mirrors: HashMap::new(),
            mirrors_policy: Policy::ContinueOnFailure,
            stores: HashMap::new(),
        }
    }

    /// Adds a Stores to the [`MultiStore`].
    pub fn add_stores(&mut self, stores: HashMap<&str, Store>) -> &mut Self {
        for (name, stores) in stores {
            self.stores.insert(name.to_string(), stores);
        }

        self
    }

    /// Sets the mirroring policy for the [`MultiStore`].
    pub fn set_mirrors_policy(&mut self, policy: Policy) -> &mut Self {
        self.mirrors_policy = policy;
        self
    }

    /// Getting single store
    pub fn get_store(&mut self, name: &str) -> Option<&Store> {
        self.stores.get(name)
    }

    /// Adds mirrors to the [`MultiStore`] with the specified name and store
    /// names.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the specified stores are not defined in the
    /// [`MultiStore`].
    pub fn add_mirrors(&mut self, name: &str, stores_names: &[&str]) -> Result<&mut Self, String> {
        let unknown_stores = stores_names
            .iter()
            .filter(|&&user_store_name| !self.stores.contains_key(user_store_name))
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>();

        if !unknown_stores.is_empty() {
            return Err(format!(
                "the stores: {} not defined",
                unknown_stores.join(",")
            ));
        };

        self.mirrors.insert(
            name.to_string(),
            stores_names.iter().map(|&s| s.to_string()).collect(),
        );
        Ok(self)
    }

    /// Creates a Mirror struct for mirroring operations from the primary store.
    #[must_use]
    pub fn mirror_stores_from_primary(&self) -> Mirror<'_> {
        let mut stores = BTreeMap::from([("primary", &self.primary)]);
        for (name, store) in &self.stores {
            stores.insert(name, store);
        }

        Mirror {
            policy: &self.mirrors_policy,
            stores,
        }
    }

    /// Mirror stores by mirror key
    #[must_use]
    pub fn mirror(&self, name: &str) -> Option<Mirror<'_>> {
        let stores_name = self.mirrors.get(name)?;
        let mut stores = BTreeMap::new();
        for (name, store) in &self.stores {
            if stores_name.contains(name) {
                stores.insert(name.as_str(), store);
            }
        }

        Some(Mirror {
            policy: &self.mirrors_policy,
            stores,
        })
    }
}

/// Struct representing a mirror for mirroring operations across multiple
/// stores.
pub struct Mirror<'a> {
    policy: &'a Policy,
    stores: BTreeMap<&'a str, &'a Store>,
}

impl<'a> Mirror<'a> {
    /// Writes content to all stores in the mirror.
    ///
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::{collections::HashMap, path::PathBuf};
    /// use active_storage::{drivers, multi_store::MultiStore, StoreConfig};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = drivers::disk::Config {
    ///         location: PathBuf::from("tmp").join("primary-storage"),
    ///     };
    ///     let disk_driver = StoreConfig::Disk(config).build().await.unwrap();
    ///
    ///     let inmem_driver = StoreConfig::InMem().build().await.unwrap();
    ///
    ///     let mut multi_store = MultiStore::new(disk_driver);
    ///     multi_store.add_stores(HashMap::from([("secondary", inmem_driver)]));
    ///
    ///     let _ = multi_store
    ///    .mirror_stores_from_primary()
    ///    .write(PathBuf::from("test").as_path(), b"content")
    ///    .await;
    /// }    
    /// ```
    ///
    /// # Errors
    ///
    /// Depend of the mirror policy return operation failure
    pub async fn write<C>(&self, path: &Path, content: C) -> MirrorResult<()>
    where
        C: AsRef<[u8]> + Send,
    {
        let mut error_stores = BTreeMap::new();
        for (name, store) in &self.stores {
            if let Err(error) = store.write(path, content.as_ref().to_vec()).await {
                self.handle_error_policy(name, error, &mut error_stores)?;
            }
        }

        if error_stores.is_empty() {
            Ok(())
        } else {
            Err(MirrorError::MirrorFailedOnStores(error_stores))
        }
    }

    /// Deletes a file from all stores in the mirror.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::{collections::HashMap, path::PathBuf};
    /// use active_storage::{drivers, multi_store::MultiStore, StoreConfig};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = drivers::disk::Config {
    ///         location: PathBuf::from("tmp").join("store-1"),
    ///     };
    ///     let disk_driver = StoreConfig::Disk(config).build().await.unwrap();
    ///
    ///     let inmem_driver = StoreConfig::InMem().build().await.unwrap();
    ///
    ///     let mut multi_store = MultiStore::new(disk_driver);
    ///     multi_store.add_stores(HashMap::from([("secondary", inmem_driver)]));
    ///
    ///     let _ = multi_store
    ///    .mirror_stores_from_primary()
    ///    .write(PathBuf::from("test").as_path(), b"content")
    ///    .await;
    ///
    ///     let _ = multi_store
    ///    .mirror_stores_from_primary()
    ///    .delete(PathBuf::from("test").as_path())
    ///    .await;
    /// }    
    /// ```
    ///
    /// # Errors
    ///
    /// Depend of the mirror policy return operation failure
    pub async fn delete(&self, path: &Path) -> MirrorResult<()> {
        let mut error_stores = BTreeMap::new();
        for (name, store) in &self.stores {
            if let Err(error) = store.delete(path).await {
                self.handle_error_policy(name, error, &mut error_stores)?;
            }
        }

        if error_stores.is_empty() {
            Ok(())
        } else {
            Err(MirrorError::MirrorFailedOnStores(error_stores))
        }
    }

    /// Deletes a directory from all stores in the mirror.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::{collections::HashMap, path::PathBuf};
    /// use active_storage::{drivers, multi_store::MultiStore, StoreConfig};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = drivers::disk::Config {
    ///         location: PathBuf::from("tmp").join("primary-storage"),
    ///     };
    ///     let disk_driver = StoreConfig::Disk(config).build().await.unwrap();
    ///
    ///     let inmem_driver = StoreConfig::InMem().build().await.unwrap();
    ///
    ///     let mut multi_store = MultiStore::new(disk_driver);
    ///     multi_store.add_stores(HashMap::from([("secondary", inmem_driver)]));
    ///
    ///     let _ = multi_store
    ///    .mirror_stores_from_primary()
    ///    .write(PathBuf::from("folder").join("file").as_path(), b"content")
    ///    .await;
    ///
    ///     let _ = multi_store
    ///    .mirror_stores_from_primary()
    ///    .delete_directory(PathBuf::from("folder").as_path())
    ///    .await;
    /// }    
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if any store fails to delete the directory.
    pub async fn delete_directory(&self, path: &Path) -> MirrorResult<()> {
        let mut error_stores = BTreeMap::new();
        for (name, store) in &self.stores {
            if let Err(error) = store.delete_directory(path).await {
                self.handle_error_policy(name, error, &mut error_stores)?;
            }
        }

        if error_stores.is_empty() {
            Ok(())
        } else {
            Err(MirrorError::MirrorFailedOnStores(error_stores))
        }
    }

    /// Handles the mirroring error policy based on the specified store's
    /// failure.
    fn handle_error_policy(
        &self,
        store_name: &str,
        error: DriverError,
        error_stores: &mut BTreeMap<String, DriverError>,
    ) -> MirrorResult<()> {
        match self.policy {
            Policy::ContinueOnFailure => {
                error_stores.insert((*store_name).to_string(), error);
                Ok(())
            }
            Policy::StopOnFailure => Err(MirrorError::MirrorFailedOnStore(
                (*store_name).to_string(),
                error,
            )),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::StoreConfig;

    #[tokio::test]
    async fn can_add_store() {
        let store = StoreConfig::InMem().build().await.unwrap();
        let mut multi_store = MultiStore::new(store);

        assert_eq!(multi_store.stores.len(), 0);

        let store_1 = StoreConfig::InMem().build().await.unwrap();
        let store_2 = StoreConfig::InMem().build().await.unwrap();

        let stores = HashMap::from([("foo", store_1), ("bar", store_2)]);

        multi_store.add_stores(stores);
        assert_eq!(multi_store.stores.len(), 2);
    }

    #[tokio::test]
    async fn can_update_policy() {
        let store = StoreConfig::InMem().build().await.unwrap();
        let mut multi_store = MultiStore::new(store);

        let init_policy = multi_store.mirrors_policy.clone();

        multi_store.set_mirrors_policy(Policy::StopOnFailure);

        assert!(init_policy != multi_store.mirrors_policy);
    }

    #[tokio::test]
    async fn can_add_mirrors() {
        let store = StoreConfig::InMem().build().await.unwrap();
        let mut multi_store = MultiStore::new(store);

        let store_1 = StoreConfig::InMem().build().await.unwrap();
        let store_2 = StoreConfig::InMem().build().await.unwrap();
        let stores = HashMap::from([("bar-store", store_1), ("baz-store", store_2)]);
        multi_store.add_stores(stores);

        assert_eq!(multi_store.mirrors.len(), 0);

        assert!(multi_store
            .add_mirrors(
                "mirror-bar-and-baz",
                vec!["bar-store", "baz-store"].as_slice()
            )
            .is_ok());

        assert_eq!(
            multi_store.mirrors.get("mirror-bar-and-baz").unwrap().len(),
            2
        );

        assert_eq!(
            multi_store
                .add_mirrors(
                    "bar",
                    vec!["baz-store", "un-existing 1", "un-existing 2"].as_slice()
                )
                .err(),
            Some("the stores: un-existing 1,un-existing 2 not defined".to_string())
        );
    }
}
