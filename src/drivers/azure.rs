use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use azure_storage::prelude::*;
use azure_storage_blobs::{blob::operations::DeleteBlobResponse, prelude::*};
use dyn_clone::DynClone;
use futures::StreamExt;

use super::{Driver, DriverError};
use crate::errors::DriverResult;

// Define a trait for Azure Storage client builders
#[async_trait::async_trait]
pub trait ClientBuilderTrait: DynClone + Sync + Send {
    async fn get_blob_content(&self, container: &str, path: &str) -> azure_core::Result<Vec<u8>>;
    async fn blob_exists(&self, container: &str, path: &str) -> azure_core::Result<bool>;
    async fn put_block_blob(
        &self,
        container: &str,
        path: &str,
        content: Vec<u8>,
    ) -> azure_core::Result<()>;
    async fn delete(&self, container: &str, path: &str) -> azure_core::Result<DeleteBlobResponse>;
    async fn get_properties(
        &self,
        container: &str,
        path: &str,
    ) -> azure_core::Result<BlobProperties>;

    async fn list_blobs(&self, container: &str) -> azure_core::Result<Vec<PathBuf>>;
}

// Define a structure representing Azure Storage client
#[derive(Clone)]
struct Client {
    client_builder: ClientBuilder,
}

// Define a structure representing Blob properties
pub struct BlobProperties {
    pub date: SystemTime,
}

// Implement the trait for the Azure Storage client builder
#[async_trait::async_trait]
impl ClientBuilderTrait for Client {
    async fn get_blob_content(&self, container: &str, path: &str) -> azure_core::Result<Vec<u8>> {
        self.client_builder
            .clone()
            .blob_client(container.to_string(), path)
            .get_content()
            .await
    }

    async fn blob_exists(&self, container: &str, path: &str) -> azure_core::Result<bool> {
        self.client_builder
            .clone()
            .blob_client(container.to_string(), path)
            .exists()
            .await
    }
    async fn put_block_blob(
        &self,
        container: &str,
        path: &str,
        content: Vec<u8>,
    ) -> azure_core::Result<()> {
        self.client_builder
            .clone()
            .blob_client(container.to_string(), path)
            .put_block_blob(content)
            .await?;
        Ok(())
    }
    async fn delete(&self, container: &str, path: &str) -> azure_core::Result<DeleteBlobResponse> {
        self.client_builder
            .clone()
            .blob_client(container.to_string(), path)
            .delete()
            .await
    }
    async fn get_properties(
        &self,
        container: &str,
        path: &str,
    ) -> azure_core::Result<BlobProperties> {
        let properties = self
            .client_builder
            .clone()
            .blob_client(container.to_string(), path)
            .get_properties()
            .await?;

        Ok(BlobProperties {
            date: properties.date.into(),
        })
    }

    async fn list_blobs(&self, container: &str) -> azure_core::Result<Vec<PathBuf>> {
        let mut paths = Vec::new();

        let mut blob_stream = self
            .client_builder
            .clone()
            .container_client(container.to_string())
            .list_blobs()
            .into_stream();

        while let Some(Ok(blob_entry)) = blob_stream.next().await {
            for blob in blob_entry.blobs.blobs() {
                paths.push(PathBuf::from(blob.name.to_string()));
            }
        }
        Ok(paths)
    }
}

#[derive(Clone)]
pub struct Config {
    pub account: String,
    pub container: String,
    pub credentials: ClientCredentials,
}

#[allow(clippy::module_name_repetitions)]
pub struct AzureDriver {
    pub container: String,
    client: Box<dyn ClientBuilderTrait>,
}

impl Clone for AzureDriver {
    fn clone(&self) -> Self {
        Self {
            container: self.container.clone(),
            client: dyn_clone::clone_box(&*self.client),
        }
    }
}

#[derive(Clone)]
pub enum ClientCredentials {
    AccessKey(String),
}

impl AzureDriver {
    /// Create a new instance of [`AzureDriver`] with the provided
    /// configuration.
    #[must_use]
    pub fn new(config: Config) -> Self {
        let storage_credentials = match config.credentials {
            ClientCredentials::AccessKey(access_key) => {
                StorageCredentials::access_key(config.account.to_string(), access_key)
            }
        };

        let client = Box::new(Client {
            client_builder: ClientBuilder::new(config.account.to_string(), storage_credentials),
        });
        Self {
            container: config.container,
            client,
        }
    }

    /// Creates a new [`AzureDriver`] instance with the provided azure client
    /// and container name.
    #[must_use]
    pub fn with_client(container: &str, client: Box<dyn ClientBuilderTrait>) -> Self {
        Self {
            container: container.to_string(),
            client,
        }
    }

    /// Get all files in a specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue listing data.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `PathBuf` representing the file paths,
    /// or an error.
    async fn get_all_files_in_path(
        &self,
        container: &str,
        dir_path: &Path,
    ) -> DriverResult<Vec<std::path::PathBuf>> {
        let prefix_folder = dir_path.to_path_buf();
        let mut paths = Vec::new();

        let container_paths = match self.client.list_blobs(container).await {
            Ok(paths) => paths,
            Err(error) => {
                return Err(error.kind().into());
            }
        };

        for path in container_paths {
            if path.starts_with(&prefix_folder) {
                paths.push(path);
            }
        }
        Ok(paths)
    }
}

#[async_trait::async_trait]
impl Driver for AzureDriver {
    /// Reads the contents of a file at the specified path within the storage.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue reading from the file or decoding
    /// its contents.
    async fn read(&self, path: &Path) -> DriverResult<Vec<u8>> {
        match self
            .client
            .get_blob_content(
                &self.container,
                path.to_str().ok_or(DriverError::InvalidPath)?,
            )
            .await
        {
            Ok(blob) => Ok(blob),
            Err(err) => return Err(err.kind().into()),
        }
    }

    /// Checks if a file exists at the specified path within the storage.
    ///
    /// If the path does not point to a file, the method returns `Ok(false)`.
    /// Otherwise, it checks if the file exists and returns the result.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue checking the existence of the
    /// file.
    async fn file_exists(&self, path: &Path) -> DriverResult<bool> {
        match self
            .client
            .blob_exists(
                &self.container,
                path.to_str().ok_or(DriverError::InvalidPath)?,
            )
            .await
        {
            Ok(is_exists) => Ok(is_exists),
            Err(err) => Err(err.kind().into()),
        }
    }

    /// Writes the provided content to a file at the specified path within the
    /// storage.
    ///
    /// If the directory structure leading to the file does not exist, it
    /// creates the necessary directories.
    ///
    /// # Errors
    ///
    /// Returns an error if there is any issue creating directories or writing
    /// to the file
    async fn write(&self, path: &Path, content: Vec<u8>) -> DriverResult<()> {
        match self
            .client
            .put_block_blob(
                &self.container,
                path.to_str().ok_or(DriverError::InvalidPath)?,
                content,
            )
            .await
        {
            Ok(()) => Ok(()),
            Err(error) => Err(error.kind().into()),
        }
    }

    /// Deletes the file at the specified path within the storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the file does not exist or if there is any issue
    /// deleting the file.
    ///
    /// If the file does not exist, the error variant
    /// `DriverError::ResourceNotFound` is returned.
    async fn delete(&self, path: &Path) -> DriverResult<()> {
        match self
            .client
            .delete(
                &self.container,
                path.to_str().ok_or(DriverError::InvalidPath)?,
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(error) => Err(error.kind().into()),
        }
    }

    /// Deletes all the files under the given path within the storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory does not exist or if there is any
    /// issue deleting the directory.
    ///
    /// If the files not found under the given path, the error variant
    /// `DriverError::DirectoryNotFound` is returned.
    async fn delete_directory(&self, path: &Path) -> DriverResult<()> {
        let paths_to_delete = self.get_all_files_in_path(&self.container, path).await?;

        if paths_to_delete.is_empty() {
            return Err(DriverError::ResourceNotFound);
        }

        for blob_path in paths_to_delete {
            if let Err(err) = self
                .client
                .delete(
                    &self.container,
                    blob_path.to_str().ok_or(DriverError::InvalidPath)?,
                )
                .await
            {
                return Err(err.kind().into());
            }
        }

        Ok(())
    }

    /// Retrieves the last modification time of the file at the specified path
    /// within the storage.
    ///
    ///  # Errors
    ///
    /// Returns an error if the file does not exist or if there is any issue
    /// retrieving the last modification time.
    ///
    /// If the file does not exist, the error variant
    /// `DriverError::ResourceNotFound` is returned.
    async fn last_modified(&self, path: &Path) -> DriverResult<SystemTime> {
        let properties = match self
            .client
            .get_properties(
                &self.container,
                path.to_str().ok_or(DriverError::InvalidPath)?,
            )
            .await
        {
            Ok(metadata) => metadata,
            Err(err) => return Err(err.kind().into()),
        };

        Ok(properties.date)
    }
}

impl From<&azure_storage::ErrorKind> for DriverError {
    fn from(kind: &azure_storage::ErrorKind) -> Self {
        match kind {
            azure_storage::ErrorKind::HttpResponse {
                status: _,
                error_code,
            } => match error_code.as_ref().map(String::as_str) {
                Some("ContainerNotFound" | "BlobNotFound") => Self::ResourceNotFound,
                Some("AuthenticationFailed") => Self::AuthenticationFailed,
                _ => Self::Any(Box::new(kind.clone().into_error())),
            },
            azure_storage::ErrorKind::Credential => Self::AuthenticationFailed,
            _ => Self::Any(Box::new(kind.clone().into_error())),
        }
    }
}
