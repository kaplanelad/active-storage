use crate::{
    drivers::Driver,
    errors::{DriverError, DriverResult},
};
use async_trait::async_trait;
use dyn_clone::DynClone;
use google_cloud_storage::{
    client::{google_cloud_auth::credentials::CredentialsFile, ClientConfig},
    http,
    http::{
        objects::{
            delete::DeleteObjectRequest,
            download::Range,
            get::GetObjectRequest,
            list::ListObjectsRequest,
            upload::{Media, UploadObjectRequest, UploadType},
            Object,
        },
        Error,
    },
};
use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

// Define a trait for the Google Cloud Storage client builders.
#[async_trait::async_trait]
pub trait ClientBuilderTrait: DynClone + Sync + Send {
    /// Download the content of an object in the Google Cloud Storage.
    async fn download_object(&self, bucket: &str, path: &str) -> Result<Vec<u8>, http::Error>;
    /// Get the metadata of an object in the Google Cloud Storage.
    async fn get_object_details(&self, bucket: &str, path: &str) -> Result<Object, http::Error>;
    /// Check if an object exists in the Google Cloud Storage.
    async fn object_exists(&self, bucket: &str, path: &str) -> Result<bool, http::Error>;
    /// Upload an object to the Google Cloud Storage.
    async fn upload_objects(
        &self,
        bucket: &str,
        path: &str,
        content: Vec<u8>,
    ) -> Result<Object, http::Error>;
    /// Delete an object from the Google Cloud Storage.
    async fn delete_objects(&self, bucket: &str, path: &str) -> Result<(), http::Error>;
    /// List all objects in the Google Cloud Storage.
    async fn list_objects(&self, bucket: &str, path: &str) -> Result<Vec<PathBuf>, http::Error>;
}

#[derive(Clone)]
pub struct Client {
    client: google_cloud_storage::client::Client,
}

impl Client {
    /// Create a new `Client` instance with the provided `ClientConfig`.
    #[must_use]
    pub fn new(config: ClientConfig) -> Self {
        Self {
            client: google_cloud_storage::client::Client::new(config),
        }
    }
}

#[async_trait]
impl ClientBuilderTrait for Client {
    async fn download_object(&self, bucket: &str, path: &str) -> Result<Vec<u8>, Error> {
        let request = &GetObjectRequest {
            bucket: bucket.to_string(),
            object: path.to_string(),
            ..Default::default()
        };
        self.client
            .download_object(request, &Range::default())
            .await
    }
    async fn get_object_details(&self, bucket: &str, path: &str) -> Result<Object, http::Error> {
        let request = &GetObjectRequest {
            bucket: bucket.to_string(),
            object: path.to_string(),
            ..Default::default()
        };
        self.client.get_object(request).await
    }

    async fn object_exists(&self, bucket: &str, path: &str) -> Result<bool, Error> {
        match self.get_object_details(bucket, path).await {
            Ok(_) => Ok(true),
            Err(e) => match e {
                Error::Response(e) => {
                    if e.code == 404 {
                        return Ok(false);
                    }
                    Err(Error::Response(e))
                }
                _ => Err(e),
            },
        }
    }

    async fn upload_objects(
        &self,
        bucket: &str,
        path: &str,
        content: Vec<u8>,
    ) -> Result<Object, Error> {
        let upload_type = UploadType::Simple(Media::new(path.to_owned()));
        let request = &UploadObjectRequest {
            bucket: bucket.to_string(),
            ..Default::default()
        };
        self.client
            .upload_object(request, content, &upload_type)
            .await
    }

    async fn delete_objects(&self, bucket: &str, path: &str) -> Result<(), Error> {
        let request = &DeleteObjectRequest {
            bucket: bucket.to_string(),
            object: path.to_string(),
            ..Default::default()
        };
        self.client.delete_object(request).await
    }

    async fn list_objects(&self, bucket: &str, path: &str) -> Result<Vec<PathBuf>, Error> {
        let request = &ListObjectsRequest {
            bucket: bucket.to_string(),
            prefix: Some(format!("{path}/")),
            ..Default::default()
        };
        let mut paths = Vec::new();
        let result = self.client.list_objects(request).await?;
        if let Some(items) = result.items {
            paths.extend(
                items
                    .into_iter()
                    .map(|item| PathBuf::from(format!("{}/{}", item.bucket, item.name))),
            );
        }
        Ok(paths)
    }
}

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
pub enum ClientCredentials {
    /// Credentials file path.
    CredentialFile(String),
}

/// The `GoogleCloudStorage` struct represents a Google Cloud Storage-based implementation of the `Driver`
/// trait.
///
/// It provides methods for interacting with files and directories on Google Cloud Storage.
pub struct GoogleCloudStorage {
    /// The Google Cloud Storage client used for communication with the service.
    client: Box<dyn ClientBuilderTrait>,
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
    pub async fn new(config: Config) -> DriverResult<Self> {
        let credential_config = if let Some(credentials) = config.credentials {
            match credentials {
                ClientCredentials::CredentialFile(path) => {
                    let credentials_file = CredentialsFile::new_from_file(path)
                        .await
                        .map_err(|_e| DriverError::InvalidPath)?;
                    ClientConfig::default()
                        .with_credentials(credentials_file)
                        .await
                        .map_err(|e| DriverError::Any(Box::new(e)))?
                }
            }
        } else {
            ClientConfig::default().anonymous()
        };
        Ok(Self {
            client: Box::new(Client::new(credential_config)),
            bucket: config.bucket,
        })
    }

    /// Creates a new `GoogleCloudStorage` instance with the provided `Client` and bucket name.
    #[must_use]
    pub fn with_client(bucket: &str, client: Box<dyn ClientBuilderTrait>) -> Self {
        Self {
            client,
            bucket: bucket.to_string(),
        }
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
    async fn get_all_files_in_path(&self, dir_path: &Path) -> DriverResult<Vec<PathBuf>> {
        let prefix_folder = dir_path.to_str().ok_or(DriverError::InvalidPath)?;
        let mut paths = Vec::new();
        let container_paths = self
            .client
            .list_objects(&self.bucket, prefix_folder)
            .await?;
        for path in container_paths {
            if path.starts_with(prefix_folder) {
                paths.push(path);
            }
        }
        Ok(paths)
    }
}

impl Clone for GoogleCloudStorage {
    fn clone(&self) -> Self {
        Self {
            client: dyn_clone::clone_box(&*self.client),
            bucket: self.bucket.clone(),
        }
    }
}

#[async_trait]
impl Driver for GoogleCloudStorage {
    /// Reads the contents of a file at the specific path within the Google Cloud Storage.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue reading from the file or decoding its contents.
    async fn read(&self, path: &Path) -> DriverResult<Vec<u8>> {
        let path = path.to_str().ok_or(DriverError::InvalidPath)?;
        let result = self.client.download_object(&self.bucket, path).await;
        match result {
            Ok(response) => Ok(response),
            Err(e) => Err(DriverError::Any(Box::new(e))),
        }
    }

    /// Checks if a file exists at the specified path within the Google Cloud Storage.
    ///
    /// If the file does not point to a file, the method returns `Ok(false)`.
    /// Otherwise, it checks if the file exists and returns the result.
    ///
    /// # Errors
    /// Returns an error if there is an issue occurs when checking existence of the file.
    async fn file_exists(&self, path: &Path) -> DriverResult<bool> {
        let path_str = path.to_str().ok_or(DriverError::InvalidPath)?;
        let result = self.client.object_exists(&self.bucket, path_str).await?;
        Ok(result)
    }

    /// Writes the provided content to a file at the specified path within the Google Cloud Storage.
    ///
    /// If the file does not exist, it is created. If the file exists, its contents are overwritten.
    ///
    /// # Errors
    ///
    /// Returns an error if there is any issue creating directories or writing to the file.
    async fn write(&self, path: &Path, content: Vec<u8>) -> DriverResult<()> {
        let path = path.to_str().ok_or(DriverError::InvalidPath)?;
        self.client
            .upload_objects(&self.bucket, path, content)
            .await?;
        Ok(())
    }

    /// Deletes the file at the specified path within the Google Cloud Storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the file does not exist or if there is any issue
    /// deleting the file.
    ///
    /// If the file does not exist, the error variant
    /// `DriverError::ResourceNotFound` is returned.
    async fn delete(&self, path: &Path) -> DriverResult<()> {
        let path = path.to_str().ok_or(DriverError::InvalidPath)?;
        self.client.delete_objects(&self.bucket, path).await?;
        Ok(())
    }

    /// Deletes all the files under the given path within the Google Cloud Storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory does not exist or if there is any
    /// issue deleting the directory.
    ///
    /// If the files not found under the given path, the error variant
    /// `DriverError::DirectoryNotFound` is returned.
    async fn delete_directory(&self, path: &Path) -> DriverResult<()> {
        let paths_to_delete = self.get_all_files_in_path(path).await?;
        if paths_to_delete.is_empty() {
            return Err(DriverError::ResourceNotFound);
        }

        for path in paths_to_delete {
            self.delete(&path).await?;
        }

        Ok(())
    }

    /// Retrieves the last modification time of the file at the specified path
    /// within the Google Cloud Storage.
    ///
    ///  # Errors
    ///
    /// Returns an error if the file does not exist or if there is any issue
    /// retrieving the last modification time.
    ///
    /// If the file does not exist, the error variant
    /// `DriverError::ResourceNotFound` is returned.
    async fn last_modified(&self, path: &Path) -> DriverResult<SystemTime> {
        let path = path.to_str().ok_or(DriverError::InvalidPath)?;
        let object = self.client.get_object_details(&self.bucket, path).await?;
        let last_modified = object
            .updated
            .unwrap_or_else(|| object.time_created.unwrap());
        Ok(SystemTime::from(last_modified))
    }
}

/// Converts an `http::Error` into a `DriverError`.
impl From<http::Error> for DriverError {
    fn from(error: http::Error) -> Self {
        match error {
            http::Error::Response(e) => {
                if e.code == 404 {
                    Self::ResourceNotFound
                } else {
                    Self::Any(Box::new(e))
                }
            }
            http::Error::HttpClient(e) => {
                // Hypothetical check for network-related errors
                if e.is_connect() || e.is_timeout() {
                    Self::Network()
                } else {
                    Self::Any(Box::new(e))
                }
            }
            http::Error::TokenSource(_e) => Self::AuthenticationFailed,
        }
    }
}
