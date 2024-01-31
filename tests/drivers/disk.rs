use active_storage::{drivers::disk::Config, StoreConfig};

use super::flow;

#[tokio::test]
async fn disk() {
    let location = tree_fs::from_yaml_str(
        r"
        files:
        ",
    )
    .unwrap();
    let config = Config {
        location: location.clone(),
    };
    // let disk_driver: Store<DiskDriver> =
    // Store::<DiskDriver>::new(config).await.unwrap();
    let disk_driver = StoreConfig::Disk(config).build().await.unwrap();

    flow::test_driver(&disk_driver, location).await;
}
