use std::path::PathBuf;

use active_storage::drivers::aws_s3::ClientCredentials;
use active_storage::{drivers, StoreConfig};

#[tokio::main]
async fn main() {
    let client_credentials = ClientCredentials {
        access_key: "your access key".to_string(),
        secret_key: "your secret key".to_string(),
        session_token: None,
    };
    let config = drivers::aws_s3::Config {
        region: "us-east-1".to_string(),
        bucket: "test-bucket".to_string(),
        credentials: Some(client_credentials),
        endpoint_url: "your endpoint url".to_string(),
    };
    let s3_driver = StoreConfig::AwsS3(config).build().await.unwrap();

    let file_path = PathBuf::from("test.txt");
    s3_driver
        .write(file_path.as_path(), b"my content")
        .await
        .unwrap();
}
