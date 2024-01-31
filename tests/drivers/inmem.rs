use std::path::PathBuf;

use active_storage::StoreConfig;

use super::flow;

#[tokio::test]
async fn inmem() {
    let inmem_driver = StoreConfig::InMem().build().await.unwrap();

    flow::test_driver(&inmem_driver, PathBuf::new()).await;
}
