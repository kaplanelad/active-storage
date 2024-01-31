use std::{
    path::{Path, PathBuf},
    str::FromStr,
    time::SystemTime,
};

use async_trait::async_trait;
use aws_sdk_s3::{
    config::Credentials,
    error::SdkError,
    primitives::ByteStream,
    types::{Delete, ObjectIdentifier},
    Client,
};
use aws_types::region::Region;

use super::{Driver, DriverError, DriverResult};
use crate::contents::Contents;

/// Configuration parameters for initializing an `AwsS3` driver instance.
pub struct Config {
    /// The name of the S3 bucket .
    pub bucket: String,
    /// The AWS region where the S3 bucket is located.
    pub region: String,
    /// Optional credentials for authenticating with the AWS S3 service.
    pub credentials: Option<ClientCredentials>,
}

/// Credentials for authenticating with the AWS S3 service.
pub struct ClientCredentials {
    pub access_key: String,
    pub secret_key: String,
    pub session_token: Option<String>,
}

/// The `AwsS3` struct represents an S3-based implementation of the `Driver`
/// trait.
///
/// It provides methods for interacting with files and directories on Amazon S3.
#[derive(Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct AwsS3 {
    /// The S3 client used for communication with the AWS service.
    client: Client,
    /// The name of the S3 bucket.
    bucket: String,
}

impl AwsS3 {
    /// Create a new instance of `AwsS3` with the provided configuration.
    ///
    /// # Returns
    ///
    /// A `Result` containing the initialized `AwsS3`.
    #[must_use]
    pub fn new(config: Config) -> Self {
        let mut client_builder = aws_sdk_s3::Config::builder()
            .force_path_style(true)
            .region(Region::new(config.region));

        if let Some(credentials) = config.credentials {
            let cred = Credentials::new(
                credentials.access_key,
                credentials.secret_key,
                credentials.session_token,
                None,
                "active-store",
            );

            client_builder = client_builder.credentials_provider(cred);
        }

        Self {
            bucket: config.bucket,
            client: Client::from_conf(client_builder.build()),
        }
    }

    /// Creates a new `AwsS3` instance with the provided S3 client and bucket
    /// name.
    #[must_use]
    pub fn with_client(client: Client, bucket: &str) -> Self {
        Self {
            client,
            bucket: bucket.to_string(),
        }
    }

    /// Get all files in a specified path on S3.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue listing s3 data.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `PathBuf` representing the file paths,
    /// or an error.
    async fn get_all_files_in_path(&self, path: &Path) -> DriverResult<Vec<std::path::PathBuf>> {
        let mut paths = Vec::new();
        let request = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(format!(
                "{}/",
                path.to_str().ok_or(DriverError::InvalidPath)?
            ));

        let mut response = request.into_paginator().send();

        while let Some(result) = response.next().await {
            let contents = match result {
                Ok(result) => result.contents.unwrap_or_default(),
                Err(error) => return Err(error.into()),
            };

            paths.extend(
                contents
                    .iter()
                    .filter_map(|content| content.key())
                    .map(|s| PathBuf::from_str(s).unwrap()),
            );
        }

        Ok(paths)
    }
}

#[async_trait]
impl Driver for AwsS3 {
    /// Reads the contents of a file at the specified path within the AWS S3
    /// storage.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue reading from the file or decoding
    /// its contents.
    async fn read(&self, path: &Path) -> DriverResult<Vec<u8>> {
        let request = match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(path.to_str().ok_or(DriverError::InvalidPath)?)
            .send()
            .await
        {
            Ok(request) => request,
            Err(error) => {
                return Err(error.into());
            }
        };

        Ok(Contents::from_bytestream(request.body)
            .await
            .map_err(|_| DriverError::DecodeError)?
            .into())
    }

    /// Checks if a file exists at the specified path within the AWS S3 storage.
    ///
    /// If the path does not point to a file, the method returns `Ok(false)`.
    /// Otherwise, it checks if the file exists and returns the result.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue checking the existence of the
    /// file.
    async fn file_exists(&self, path: &Path) -> DriverResult<bool> {
        let request = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(path.to_str().ok_or(DriverError::InvalidPath)?)
            .send()
            .await;

        match request {
            Ok(_) => Ok(true),
            Err(SdkError::ServiceError(error)) => {
                if error.err().is_not_found() {
                    return Ok(false);
                }

                Err(SdkError::ServiceError(error).into())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Writes the provided content to a file at the specified path within the
    /// AWS S3 storage.
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
            .put_object()
            .bucket(&self.bucket)
            .key(path.to_str().ok_or(DriverError::InvalidPath)?)
            .body(ByteStream::from(content))
            .send()
            .await
        {
            Ok(_put) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    /// Deletes the file at the specified path within the AWS S3 storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the file does not exist or if there is any issue
    /// deleting the file.
    ///
    /// If the file does not exist, the error variant
    /// `DriverError::ResourceNotFound` is returned.
    async fn delete(&self, path: &Path) -> DriverResult<()> {
        if !self.file_exists(path).await? {
            return Err(DriverError::ResourceNotFound);
        }

        if let Err(err) = self
            .client
            .delete_object()
            .bucket(&self.bucket)
            .key(path.to_str().ok_or(DriverError::InvalidPath)?)
            .send()
            .await
        {
            return Err(err.into());
        }

        Ok(())
    }

    /// Deletes all the files under the given path within the AWS S3 storage.
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

        if let Err(err) = self
            .client
            .delete_objects()
            .bucket(&self.bucket)
            .delete(
                Delete::builder()
                    .set_objects(Some(
                        paths_to_delete
                            .iter()
                            .map(|path| {
                                ObjectIdentifier::builder()
                                    .key(path.to_str().unwrap().to_string())
                                    .build()
                                    .unwrap()
                            })
                            .collect(),
                    ))
                    .build()
                    .unwrap(),
            )
            .send()
            .await
        {
            return Err(err.into());
        }

        Ok(())
    }

    /// Retrieves the last modification time of the file at the specified path
    /// within the AWS S3 storage. # Errors
    ///
    /// Returns an error if the file does not exist or if there is any issue
    /// retrieving the last modification time.
    ///
    /// If the file does not exist, the error variant
    /// `DriverError::ResourceNotFound` is returned.
    async fn last_modified(&self, path: &Path) -> DriverResult<SystemTime> {
        let response = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(path.to_str().ok_or(DriverError::InvalidPath)?)
            .send()
            .await;

        match response {
            Ok(response) => Ok(SystemTime::try_from(
                response
                    .last_modified
                    .ok_or(DriverError::Any("last modify is missing".into()))?,
            )
            .map_err(Box::from)?),
            Err(e) => Err(e.into()),
        }
    }
}

// Errors conventions
type AwsApiError<T> = aws_smithy_runtime_api::client::result::SdkError<
    T,
    aws_smithy_runtime_api::client::orchestrator::HttpResponse,
>;

impl From<AwsApiError<aws_sdk_s3::operation::get_object::GetObjectError>> for DriverError {
    fn from(kind: AwsApiError<aws_sdk_s3::operation::get_object::GetObjectError>) -> Self {
        match kind {
            aws_smithy_runtime_api::client::result::SdkError::ConstructionFailure(_)
            | aws_smithy_runtime_api::client::result::SdkError::TimeoutError(_)
            | aws_smithy_runtime_api::client::result::SdkError::DispatchFailure(_) => {
                Self::Network()
            }
            aws_smithy_runtime_api::client::result::SdkError::ResponseError(err) => {
                let raw = err.raw();
                if raw.status().as_u16() == 404 {
                    Self::ResourceNotFound
                } else {
                    Self::Network()
                }
            }
            aws_smithy_runtime_api::client::result::SdkError::ServiceError(err) => {
                match err.err() {
                    aws_sdk_s3::operation::get_object::GetObjectError::NoSuchKey(_) => {
                        Self::ResourceNotFound
                    }
                    _ => Self::Any(err.err().to_string().into()),
                }
            }
            _ => Self::Any(Box::new(kind) as Box<_>),
        }
    }
}

impl From<AwsApiError<aws_sdk_s3::operation::head_object::HeadObjectError>> for DriverError {
    fn from(kind: AwsApiError<aws_sdk_s3::operation::head_object::HeadObjectError>) -> Self {
        match kind {
            aws_smithy_runtime_api::client::result::SdkError::ConstructionFailure(_)
            | aws_smithy_runtime_api::client::result::SdkError::TimeoutError(_)
            | aws_smithy_runtime_api::client::result::SdkError::DispatchFailure(_) => {
                Self::Network()
            }
            aws_smithy_runtime_api::client::result::SdkError::ResponseError(err) => {
                let raw = err.raw();
                if raw.status().as_u16() == 404 {
                    Self::ResourceNotFound
                } else {
                    Self::Network()
                }
            }
            aws_smithy_runtime_api::client::result::SdkError::ServiceError(err) => {
                match err.err() {
                    aws_sdk_s3::operation::head_object::HeadObjectError::NotFound(_) => {
                        Self::ResourceNotFound
                    }
                    _ => Self::Any(err.err().to_string().into()),
                }
            }
            _ => Self::Any(Box::new(kind)),
        }
    }
}

impl From<AwsApiError<aws_sdk_s3::operation::put_object::PutObjectError>> for DriverError {
    fn from(kind: AwsApiError<aws_sdk_s3::operation::put_object::PutObjectError>) -> Self {
        match kind {
            aws_smithy_runtime_api::client::result::SdkError::ConstructionFailure(_)
            | aws_smithy_runtime_api::client::result::SdkError::TimeoutError(_)
            | aws_smithy_runtime_api::client::result::SdkError::DispatchFailure(_) => {
                Self::Network()
            }
            aws_smithy_runtime_api::client::result::SdkError::ResponseError(err) => {
                let raw = err.raw();
                if raw.status().as_u16() == 404 {
                    Self::ResourceNotFound
                } else {
                    Self::Network()
                }
            }
            aws_smithy_runtime_api::client::result::SdkError::ServiceError(err) => {
                Self::Any(err.err().to_string().into())
            }
            _ => Self::Any(Box::new(kind)),
        }
    }
}

impl From<AwsApiError<aws_sdk_s3::operation::delete_object::DeleteObjectError>> for DriverError {
    fn from(kind: AwsApiError<aws_sdk_s3::operation::delete_object::DeleteObjectError>) -> Self {
        match kind {
            aws_smithy_runtime_api::client::result::SdkError::ConstructionFailure(_)
            | aws_smithy_runtime_api::client::result::SdkError::TimeoutError(_)
            | aws_smithy_runtime_api::client::result::SdkError::DispatchFailure(_) => {
                Self::Network()
            }
            aws_smithy_runtime_api::client::result::SdkError::ResponseError(err) => {
                let raw = err.raw();
                if raw.status().as_u16() == 404 {
                    Self::ResourceNotFound
                } else {
                    Self::Network()
                }
            }
            aws_smithy_runtime_api::client::result::SdkError::ServiceError(err) => {
                Self::Any(err.err().to_string().into())
            }
            _ => Self::Any(Box::new(kind)),
        }
    }
}

impl From<AwsApiError<aws_sdk_s3::operation::delete_objects::DeleteObjectsError>> for DriverError {
    fn from(kind: AwsApiError<aws_sdk_s3::operation::delete_objects::DeleteObjectsError>) -> Self {
        match kind {
            aws_smithy_runtime_api::client::result::SdkError::ConstructionFailure(_)
            | aws_smithy_runtime_api::client::result::SdkError::TimeoutError(_)
            | aws_smithy_runtime_api::client::result::SdkError::DispatchFailure(_) => {
                Self::Network()
            }
            aws_smithy_runtime_api::client::result::SdkError::ResponseError(err) => {
                let raw = err.raw();
                if raw.status().as_u16() == 404 {
                    Self::ResourceNotFound
                } else {
                    Self::Network()
                }
            }
            aws_smithy_runtime_api::client::result::SdkError::ServiceError(err) => {
                Self::Any(err.err().to_string().into())
            }
            _ => Self::Any(Box::new(kind)),
        }
    }
}

impl From<AwsApiError<aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Error>> for DriverError {
    fn from(kind: AwsApiError<aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Error>) -> Self {
        match kind {
            aws_smithy_runtime_api::client::result::SdkError::ConstructionFailure(_)
            | aws_smithy_runtime_api::client::result::SdkError::TimeoutError(_)
            | aws_smithy_runtime_api::client::result::SdkError::DispatchFailure(_) => {
                Self::Network()
            }
            aws_smithy_runtime_api::client::result::SdkError::ResponseError(err) => {
                let raw = err.raw();
                if raw.status().as_u16() == 404 {
                    Self::ResourceNotFound
                } else {
                    Self::Network()
                }
            }
            aws_smithy_runtime_api::client::result::SdkError::ServiceError(err) => {
                match err.err() {
                    aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Error::NoSuchBucket(_) => {
                        Self::ResourceNotFound
                    }
                    _ => Self::Any(err.err().to_string().into()),
                }
            }
            _ => Self::Any(Box::new(kind)),
        }
    }
}
