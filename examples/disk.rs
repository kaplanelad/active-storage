use std::path::PathBuf;

use active_storage::{drivers, StoreConfig};

#[tokio::main]
async fn main() {
    let config = drivers::disk::Config {
        location: PathBuf::from("tmp"),
    };
    let disk_driver = StoreConfig::Disk(config).build().await.unwrap();

    let file_path = PathBuf::from("test.txt");
    disk_driver
        .write(file_path.as_path(), "my content")
        .await
        .unwrap();
}
