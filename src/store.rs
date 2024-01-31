use std::path::Path;

use crate::{
    contents::Contents,
    drivers::Driver,
    errors::{DriverError, DriverResult},
};
pub struct Store {
    driver: Box<dyn Driver>,
}

impl Clone for Store {
    fn clone(&self) -> Self {
        Self {
            driver: dyn_clone::clone_box(&*self.driver),
        }
    }
}

impl Store {
    #[must_use]
    pub fn new(driver: Box<dyn Driver>) -> Self {
        Self { driver }
    }
    /// Checks if a file exists at the specified path within the storage.
    ///
    /// # Parameters
    ///
    /// - `path`: The path to the file to be checked.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use active_storage::StoreConfig;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let inmem_driver = StoreConfig::InMem().build().await.unwrap();
    ///     let file_path = PathBuf::from("test.txt");
    ///     inmem_driver.write(file_path.as_path(), "my content").await;
    ///     assert!(inmem_driver.file_exists(file_path.as_path()).await.unwrap());
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying `Driver` encounters an issue while
    /// checking file existence.
    pub async fn file_exists(&self, path: &Path) -> DriverResult<bool> {
        self.driver.file_exists(path).await
    }

    /// Writes the provided contents to a file at the specified path within the
    /// storage.
    ///
    /// # Parameters
    ///
    /// - `path`: The path to the file to be written.
    /// - `contents`: The contents to be written to the file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use active_storage::StoreConfig;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let inmem_driver = StoreConfig::InMem().build().await.unwrap();
    ///     let file_path = PathBuf::from("test.txt");
    ///     assert!(inmem_driver.write(file_path.as_path(), "my content").await.is_ok());
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying `Driver` encounters an issue while
    /// writing to the file.
    pub async fn write<C: AsRef<[u8]> + Send>(&self, path: &Path, content: C) -> DriverResult<()> {
        self.driver.write(path, content.as_ref().to_vec()).await
    }

    /// Reads the contents of a file at the specified path within the storage.
    ///
    /// # Parameters
    ///
    /// - `path`: The path to the file to be read.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use active_storage::StoreConfig;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let inmem_driver = StoreConfig::InMem().build().await.unwrap();
    ///     let file_path = PathBuf::from("test.txt");
    ///     inmem_driver.write(file_path.as_path(), "my content").await;
    ///     assert_eq!(
    ///         inmem_driver.read::<String>(file_path.as_path()).await.unwrap(),
    ///         "my content".to_string(),
    ///     );
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying `Driver` encounters an issue while
    /// reading from the file.
    pub async fn read<T: TryFrom<Contents>>(&self, path: &Path) -> DriverResult<T> {
        Contents::from(self.driver.read(path).await?)
            .try_into()
            .map_or_else(|_| Err(DriverError::DecodeError), |content| Ok(content))
    }

    /// Deletes a file at the specified path within the storage.
    ///
    /// # Parameters
    ///
    /// - `path`: The path to the file to be deleted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use active_storage::StoreConfig;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let inmem_driver = StoreConfig::InMem().build().await.unwrap();
    ///     let file_path = PathBuf::from("test.txt");
    ///     inmem_driver.write(file_path.as_path(), "my content").await;
    ///     assert!(inmem_driver.delete(file_path.as_path()).await.is_ok());
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying `Driver` encounters an issue while
    /// deleting the file.
    pub async fn delete(&self, path: &Path) -> DriverResult<()> {
        self.driver.delete(path).await
    }

    /// Deletes a directory at the specified path within the storage.
    ///
    /// # Parameters
    ///
    /// - `path`: The path to the directory to be deleted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use active_storage::StoreConfig;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let inmem_driver = StoreConfig::InMem().build().await.unwrap();
    ///     let folder = PathBuf::from("foo");
    ///     let file_path = folder.join("bar.txt");
    ///     inmem_driver.write(file_path.as_path(), "my content").await;
    ///     assert!(inmem_driver.delete_directory(folder.as_path()).await.is_ok());
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying `Driver` encounters an issue while
    /// deleting the directory.
    pub async fn delete_directory(&self, path: &Path) -> DriverResult<()> {
        self.driver.delete_directory(path).await
    }

    /// Retrieves the last modified timestamp of a file at the specified path
    /// within the storage.
    ///
    /// # Parameters
    ///
    /// - `path`: The path to the file for which the last modified timestamp is
    ///   retrieved.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use active_storage::StoreConfig;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let inmem_driver = StoreConfig::InMem().build().await.unwrap();
    ///     let file_path = PathBuf::from("test.txt");
    ///     println!("{:#?}", inmem_driver.last_modified(file_path.as_path()).await);
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying `Driver` encounters an issue while
    /// retrieving the timestamp.
    pub async fn last_modified(&self, path: &Path) -> DriverResult<std::time::SystemTime> {
        self.driver.last_modified(path).await
    }
}
