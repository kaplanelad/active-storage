use active_storage::{drivers, StoreConfig};

#[tokio::main]
async fn main() {
    let config = active_storage::drivers::gcp_storage::Config {
        bucket: "bucket".to_string(),
        project_id: "project_id".to_string(),
        credentials: Some(drivers::gcp_storage::ClientCredentials::CredentialFile("path/to/credentials.json".to_string())),
    };
    let gcp_driver = StoreConfig::Gcp(config).build().await.unwrap();

    let file_path = std::path::PathBuf::from("test1.txt");
    gcp_driver
        .write(file_path.as_path(), "my content")
        .await
        .unwrap();

}