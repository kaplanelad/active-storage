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
