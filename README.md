[![Current Crates.io Version](https://img.shields.io/crates/v/active-storage.svg)](https://crates.io/crates/active-storage)

# Active Storage

Active Storage streamlines the process of uploading files to cloud storage, offering both local disk-based and in-memory services for development and testing. Additionally, it supports mirroring files to subordinate services, enhancing capabilities for backups and migrations.

It's inspired by Rails [Active Store](https://guides.rubyonrails.org/active_storage_overview.html)


## Services

* [Disk](./examples/disk.rs)
* [In Memory](./examples/in_memory.rs)
* [AWS S3](./examples/aws_s3.rs) - Requires enabling the `aws_s3` feature.


## Single Store Usage Example

```rust
use std::path::PathBuf;
use active_storage::{drivers, StoreConfig};

#[tokio::main]
async fn main() {
    let config = drivers::aws_s3::Config {
        region: "us-east-1".to_string(),
        bucket: "test-bucket".to_string(),
        credentials: None,
    };
    let s3_driver = StoreConfig::AwsS3(config).build().await.unwrap();

    let file_path = PathBuf::from("test.txt");
    s3_driver
        .write(file_path.as_path(), b"my content")
        .await
        .unwrap();
}

```


## Mirroring Usage Example

```rust
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

```


