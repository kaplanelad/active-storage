use std::path::{Path, PathBuf};

use active_storage::{errors::DriverError, store::Store};

/// Tests various functionalities of a generic `Driver` implementation.
///
/// The test includes writing and reading files, deleting files and directories,
/// checking file existence, and verifying last modified timestamps.
///
/// # Parameters
///
/// - `driver`: A reference to a `Store` containing a generic `Driver`
///   implementation.
/// - `location`: The base path for testing operations.
pub async fn test_driver(driver: &Store, location: PathBuf) {
    let foo_directory = location.join("foo");
    let foo_directory_file_1 = foo_directory.join("foo_file-1.txt");

    assert_unknown_files(driver).await;

    assert_write_file(driver, &foo_directory_file_1).await;

    assert_last_modified(driver, location.as_path()).await;

    assert_delete_file(driver, foo_directory_file_1.as_path()).await;

    assert_directories(driver, location.as_path()).await;
}

/// Asserts behaviors related to writing a file, checking its existence, and
/// reading its content.
///
/// This function tests a generic `Driver` implementation's behavior when
/// writing a file, checking its existence, and reading its content. It verifies
/// that the file is successfully written, can be found, and its content matches
/// the expected value.
async fn assert_write_file(driver: &Store, file: &Path) {
    // write a file
    assert!(
        driver.write(file, b"content").await.is_ok(),
        "file should be written"
    );

    // file should found
    assert!(
        driver.file_exists(file).await.unwrap(),
        "file should be found"
    );

    // read file content
    assert_eq!(
        driver.read::<String>(file).await.unwrap(),
        "content".to_string(),
        "invalid file content"
    );
}

/// Asserts behaviors related to deleting a file.
///
/// This function tests a generic `Driver` implementation's behavior when
/// deleting a file. It verifies that the file is successfully deleted, and
/// subsequent checks for its existence return false.
async fn assert_delete_file(driver: &Store, path: &Path) {
    //delete file
    assert!(driver.delete(path).await.is_ok(), "file should be deleted");

    // file should not exists after deletion
    assert!(
        !driver.file_exists(path).await.unwrap(),
        "file should not exist after deletion"
    );
}

/// Asserts behaviors related to the last modified timestamp of a file.
async fn assert_last_modified(driver: &Store, path: &Path) {
    let file = path.join("file.txt");
    // write a file
    assert!(
        driver.write(file.as_path(), b"content").await.is_ok(),
        "file should be written"
    );

    assert!(
        driver
            .last_modified(file.as_path())
            .await
            .unwrap()
            .elapsed()
            .unwrap()
            .as_secs()
            < 1,
        "last modified file should be less then 1 second"
    );
}

/// Asserts behaviors related to directories, file creation, and deletion.
///
/// This function tests a generic `Driver` implementation's behavior when
/// creating, writing, and deleting directories and files. It verifies that
/// files under deleted directories are also deleted while files in other
/// directories remain unaffected.
async fn assert_directories(driver: &Store, path: &Path) {
    let foo_directory = path.join("foo");
    let bar_directory = path.join("bar");
    let foo_directory_file_1 = foo_directory.join("foo_file-1.txt");
    let bar_directory_file_1 = bar_directory.join("bar_file-1.txt");
    let bar_directory_file_2 = bar_directory.join("bar_file-2.txt");

    driver
        .write(foo_directory_file_1.as_path(), b"content")
        .await
        .unwrap();

    // crate bar file 1 under bar folder
    driver
        .write(bar_directory_file_1.as_path(), b"content")
        .await
        .unwrap();

    // crate bar file 2 under bar folder
    driver
        .write(bar_directory_file_2.as_path(), b"content")
        .await
        .unwrap();

    // delete bar directory
    assert!(
        driver
            .delete_directory(bar_directory.as_path())
            .await
            .is_ok(),
        "expected bar directory to be deleted"
    );

    // foo file should be exits
    assert!(
        driver
            .file_exists(foo_directory_file_1.as_path())
            .await
            .unwrap(),
        "foo file should be exists after bar directory deletion"
    );

    // bar files under bar folder should be deleted
    assert!(
        !driver
            .file_exists(bar_directory_file_1.as_path())
            .await
            .unwrap(),
        "bar file should be deleted after bar directory deletion"
    );

    assert!(
        !driver
            .file_exists(bar_directory_file_2.as_path())
            .await
            .unwrap(),
        "bar file should be deleted after bar directory deletion"
    );
}

/// Asserts behaviors related to unknown files and directories.
///
/// This function checks the behavior of a generic `Driver` implementation when
/// interacting with files and directories that do not exist. It verifies that
/// file existence checks return false, and deletion attempts result in specific
/// `DriverError` variants.
async fn assert_unknown_files(driver: &Store) {
    let path = PathBuf::from("unknown").join("file.txt");
    // file should not exists
    assert!(
        !driver.file_exists(path.as_path()).await.unwrap(),
        "file should not exists"
    );

    // validate error when deleting file that doesn't exist
    assert!(matches!(
        driver.delete(path.as_path()).await,
        Err(DriverError::ResourceNotFound)
    ));

    // validate error when deleting file that doesn't exist
    assert!(matches!(
        driver.delete_directory(path.as_path()).await,
        Err(DriverError::ResourceNotFound)
    ));

    // validate error when deleting file that doesn't exist
    assert!(matches!(
        driver.last_modified(path.as_path()).await,
        Err(DriverError::ResourceNotFound)
    ));
}
