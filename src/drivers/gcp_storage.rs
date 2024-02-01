use std::path::{Path, PathBuf};
use std::time::SystemTime;
use async_trait::async_trait;
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::client::google_cloud_auth::credentials::CredentialsFile;
use google_cloud_storage::http::objects::list::ListObjectsRequest;
use crate::drivers::Driver;
use crate::errors::{DriverError, DriverResult};


/// Configuration parameters for initializing a `GoogleCloudStorage` driver instance.
pub struct Config {
    /// The name of the Google Cloud Storage bucket.
    pub bucket: String,
    /// The project ID associated with the Google Cloud Storage.
    pub project_id: String,
    /// Optional credentials for authenticating with the Google Cloud Storage service.
    pub credentials: Option<ClientCredentials>,
}

/// Credentials for authenticating with the Google Cloud Storage service.
pub struct ClientCredentials {
    /// Credentials file path.
    pub credentials_file: String,
}


/// The `GoogleCloudStorage` struct represents a Google Cloud Storage-based implementation of the `Driver`
/// trait.
///
/// It provides methods for interacting with files and directories on Google Cloud Storage.
#[derive(Clone)]
pub struct GoogleCloudStorage {
    /// The Google Cloud Storage client used for communication with the service.
    client: Client,
    /// The name of the Google Cloud Storage bucket.
    bucket: String,
}

impl GoogleCloudStorage {
    /// Initializes a new `GoogleCloudStorage` instance with the specified configuration.
    ///
    /// # Errors
    ///
    /// If the credentials file is not found, returns an `DriverError::InvalidPath` error.
    /// If the credentials file is invalid, returns an `DriverError::Any` error.
    ///
    /// # Returns
    ///
    /// A `Result` containing the initialized `GoogleCloudStorage`.
    /// Returns an error if the initialization fails.
    #[must_use]
    pub async fn new(config: Config) -> DriverResult<Self> {
        let credential_config = if let Some(credentials) = config.credentials {
            let credentials_file = CredentialsFile::new_from_file(credentials.credentials_file).await.map_err(|e| DriverError::InvalidPath)?;
            ClientConfig::default().with_credentials(credentials_file).await.map_err(|e| DriverError::Any(Box::new(e)))?
        } else {
            ClientConfig::default().anonymous()
        };
        Ok(Self { client: Client::new(credential_config), bucket: config.bucket })
    }

    /// Creates a new `GoogleCloudStorage` instance with the provided `Client` and bucket name.
    #[must_use]
    pub fn with_client(client: Client, bucket: &str) -> Self {
        Self { client, bucket: bucket.to_string() }
    }

    /// Get all files in a specified path on Google Cloud Storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the path is invalid or issue occurs when listing files.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `PathBuf` representing the file paths,
    /// or an error
    async fn get_all_files_in_path(&self, path: &Path) -> DriverResult<Vec<PathBuf>> {
        let mut paths = Vec::new();
        let path = path.to_str().ok_or(DriverError::InvalidPath)?;
        let result = self.client.list_objects(&ListObjectsRequest {
            bucket: self.bucket.to_string(),
            prefix: Some(format!("{}/", path)),
            ..Default::default()
        }).await;
        match result {
            Ok(response) => {
                if let Some(items) = response.items {
                    paths.extend(items.into_iter().map(|item| PathBuf::from(format!("{}/{}", item.bucket, item.name))));
                }
            }
            Err(e) => {
                return Err(DriverError::Any(Box::new(e)));
            }
        }
        Ok(paths)
    }
}

#[async_trait]
impl Driver for GoogleCloudStorage{
    async fn read(&self, path: &Path) -> DriverResult<Vec<u8>> {
        todo!()
    }

    async fn file_exists(&self, path: &Path) -> DriverResult<bool> {
        todo!()
    }

    async fn write(&self, path: &Path, content: Vec<u8>) -> DriverResult<()> {
        todo!()
    }

    async fn delete(&self, path: &Path) -> DriverResult<()> {
        todo!()
    }

    async fn delete_directory(&self, path: &Path) -> DriverResult<()> {
        todo!()
    }

    async fn last_modified(&self, path: &Path) -> DriverResult<SystemTime> {
        todo!()
    }
}