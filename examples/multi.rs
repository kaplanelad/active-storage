use std::{collections::HashMap, path::PathBuf};

use active_storage::{drivers, multi_store::MultiStore, StoreConfig};

#[tokio::main]
async fn main() {
    let config = drivers::disk::Config {
        location: PathBuf::from("tmp").join("primary-storage"),
    };
    let store_one = StoreConfig::Disk(config).build().await.unwrap();

    let config = drivers::disk::Config {
        location: PathBuf::from("tmp").join("backups"),
    };
    let secondary_store = StoreConfig::Disk(config).build().await.unwrap();

    let mut multi_store = MultiStore::new(store_one);
    multi_store.add_stores(HashMap::from([("secondary", secondary_store)]));

    let _ = multi_store
        .mirror_stores_from_primary()
        .write(PathBuf::from("test").as_path(), b"content")
        .await;
}
