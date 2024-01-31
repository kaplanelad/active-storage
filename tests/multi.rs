use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use active_storage::{
    drivers,
    multi_store::{MultiStore, Policy},
    StoreConfig,
};
use insta::assert_debug_snapshot;
use rstest::rstest;

fn create_temp_folder() -> PathBuf {
    tree_fs::from_yaml_str(
        r"
        files:
        ",
    )
    .unwrap()
}

async fn init_multi_store(root_path_store_1: &Path, root_path_store_2: &Path) -> MultiStore {
    let config = drivers::disk::Config {
        location: root_path_store_1.join("store-1"),
    };

    let store_one = StoreConfig::Disk(config).build().await.unwrap();

    let config = drivers::disk::Config {
        location: root_path_store_2.join("store-2"),
    };
    let secondary_store = StoreConfig::Disk(config).build().await.unwrap();

    let mut multi_store = MultiStore::new(store_one);
    multi_store.add_stores(HashMap::from([("store-2", secondary_store)]));

    multi_store
}

#[tokio::test]
async fn can_write_with_mirror_from_master() {
    let root_location_first_store = create_temp_folder();
    let root_location_second_store = create_temp_folder();

    let multi_store =
        init_multi_store(&root_location_first_store, &root_location_second_store).await;

    assert!(multi_store
        .mirror_stores_from_primary()
        .write(PathBuf::from("test").as_path(), b"content")
        .await
        .is_ok());

    let store_1_file_path = root_location_first_store.join("store-1").join("test");
    let store_2_file_path = root_location_second_store.join("store-2").join("test");

    assert!(store_1_file_path.exists());

    assert!(store_2_file_path.exists());

    assert_eq!(
        fs::read_to_string(store_1_file_path).unwrap(),
        fs::read_to_string(store_2_file_path).unwrap()
    );
}

#[tokio::test]
async fn can_delete_with_mirror_from_master() {
    let root_location_first_store = create_temp_folder();
    let root_location_second_store = create_temp_folder();

    let multi_store =
        init_multi_store(&root_location_first_store, &root_location_second_store).await;

    assert!(multi_store
        .mirror_stores_from_primary()
        .write(PathBuf::from("test").as_path(), b"content")
        .await
        .is_ok());

    let store_1_file_path = root_location_first_store.join("store-1").join("test");
    let store_2_file_path = root_location_second_store.join("store-2").join("test");

    assert!(store_1_file_path.exists());
    assert!(store_2_file_path.exists());

    assert!(multi_store
        .mirror_stores_from_primary()
        .delete(PathBuf::from("test").as_path())
        .await
        .is_ok());

    assert!(!store_1_file_path.exists());
    assert!(!store_2_file_path.exists());
}

#[tokio::test]
async fn can_delete_directory_with_mirror_from_master() {
    let root_location_first_store = create_temp_folder();
    let root_location_second_store = create_temp_folder();

    let multi_store =
        init_multi_store(&root_location_first_store, &root_location_second_store).await;

    assert!(multi_store
        .mirror_stores_from_primary()
        .write(
            PathBuf::from("delete-folder").join("test").as_path(),
            b"content"
        )
        .await
        .is_ok());

    let store_1_file_path = root_location_first_store
        .join("store-1")
        .join("delete-folder")
        .join("test");
    let store_2_file_path = root_location_second_store
        .join("store-2")
        .join("delete-folder")
        .join("test");

    assert!(store_1_file_path.exists());
    assert!(store_2_file_path.exists());

    assert!(multi_store
        .mirror_stores_from_primary()
        .delete_directory(PathBuf::from("delete-folder").as_path())
        .await
        .is_ok());

    assert!(!store_1_file_path.exists());
    assert!(!store_2_file_path.exists());
}

#[rstest]
#[case(Policy::ContinueOnFailure)]
#[case(Policy::StopOnFailure)]
#[tokio::test]
async fn can_failure_policy(#[case] policy: Policy) {
    let root_location_first_store = create_temp_folder();
    let root_location_second_store = create_temp_folder();
    let root_location_third_store = create_temp_folder();

    let config = drivers::disk::Config {
        location: root_location_first_store.join("store-1"),
    };

    let first_store = StoreConfig::Disk(config).build().await.unwrap();

    let config = drivers::disk::Config {
        location: root_location_second_store.join("store-2"),
    };
    let second_store = StoreConfig::Disk(config).build().await.unwrap();

    let config = drivers::disk::Config {
        location: root_location_third_store.join("store-3"),
    };
    let third_store = StoreConfig::Disk(config).build().await.unwrap();

    let mut multi_store = MultiStore::new(first_store);
    multi_store.set_mirrors_policy(policy.clone());
    multi_store.add_stores(HashMap::from([
        ("store-2", second_store),
        ("store-3", third_store),
    ]));

    assert!(multi_store
        .mirror_stores_from_primary()
        .write(PathBuf::from("test").as_path(), b"content")
        .await
        .is_ok());

    assert!(fs::remove_dir_all(root_location_second_store).is_ok());
    assert!(fs::remove_dir_all(root_location_third_store).is_ok());

    let res = multi_store
        .mirror_stores_from_primary()
        .delete(PathBuf::from("test").as_path())
        .await;

    assert_debug_snapshot!(format!("{policy:?}"), res);
}

#[tokio::test]
async fn can_mirror() {
    let root_location_first_store = create_temp_folder();
    let root_location_second_store = create_temp_folder();
    let root_location_third_store = create_temp_folder();

    let config = drivers::disk::Config {
        location: root_location_first_store.join("store-1"),
    };

    let first_store = StoreConfig::Disk(config).build().await.unwrap();

    let config = drivers::disk::Config {
        location: root_location_second_store.join("store-2"),
    };
    let second_store = StoreConfig::Disk(config).build().await.unwrap();

    let config = drivers::disk::Config {
        location: root_location_third_store.join("store-3"),
    };
    let third_store = StoreConfig::Disk(config).build().await.unwrap();

    let mut multi_store = MultiStore::new(first_store);
    multi_store.add_stores(HashMap::from([
        ("store-2", second_store),
        ("store-3", third_store),
    ]));
    assert!(multi_store
        .add_mirrors("foo", vec!["store-2", "store-3"].as_slice())
        .is_ok());

    assert!(multi_store
        .mirror("foo")
        .unwrap()
        .write(PathBuf::from("test").as_path(), b"content")
        .await
        .is_ok());

    assert!(!root_location_first_store
        .join("store-1")
        .join("test")
        .exists());
    assert!(root_location_second_store
        .join("store-2")
        .join("test")
        .exists());
    assert!(root_location_third_store
        .join("store-3")
        .join("test")
        .exists());
}
