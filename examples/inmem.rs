use std::path::PathBuf;

use active_storage::StoreConfig;

#[tokio::main]
async fn main() {
    let inmem_driver = StoreConfig::InMem().build().await.unwrap();
    let file_path = PathBuf::from("test.txt");
    inmem_driver
        .write(file_path.as_path(), "my content")
        .await
        .unwrap();

    let read_content: String = inmem_driver.read(file_path.as_path()).await.unwrap();
    println!("{read_content}");
}
