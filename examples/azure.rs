use std::path::PathBuf;

use active_storage::{drivers, StoreConfig};

#[tokio::main]
async fn main() {
    let config = drivers::azure::Config {
        account: "account".to_string(),
        container: "test".to_string(),
        credentials: drivers::azure::ClientCredentials::AccessKey("key".to_string()),
    };
    let azure_driver = StoreConfig::Azure(config).build().await.unwrap();

    let file_path = PathBuf::from("test1.txt");
    azure_driver
        .write(file_path.as_path(), "my content")
        .await
        .unwrap();
}
