use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    time::SystemTime,
};

use async_trait::async_trait;
use tokio::fs;

use super::{Driver, DriverError, DriverResult};
use crate::contents::Contents;

/// Configuration parameters for initializing a `DiskDriver`.
pub struct Config {
    pub location: PathBuf,
}

/// The `DiskDriver` struct represents a disk-based implementation of the
/// `Driver` trait.
///
/// It provides methods for interacting with files and directories on the disk.
#[derive(Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct DiskDriver {
    /// The location on the disk where the `DiskDriver` will operate.
    location: PathBuf,
}

impl From<ErrorKind> for DriverError {
    fn from(kind: ErrorKind) -> Self {
        match kind {
            ErrorKind::NotFound => Self::ResourceNotFound,
            _ => kind.into(),
        }
    }
}

impl DiskDriver {
    /// Initializes a new `DiskDriver` instance with the specified
    /// configuration.
    ///
    /// If the specified location does not exist, it creates the necessary
    /// directories.
    ///
    /// # Errors
    ///
    /// Returns an error if the initialization fails, such as being unable to
    /// create the required directories.
    pub async fn new(config: Config) -> DriverResult<Self> {
        if !config.location.exists() {
            if let Err(err) = fs::create_dir_all(&config.location).await {
                return Err(err.kind().into());
            }
        }

        Ok(Self {
            location: config.location,
        })
    }
}

#[async_trait]
impl Driver for DiskDriver {
    /// Reads the contents of a file at the specified path within the disk-based
    /// storage.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue reading from the file or decoding
    /// its contents.
    async fn read(&self, path: &Path) -> DriverResult<Vec<u8>> {
        let path = self.location.join(path);

        let content = match fs::read(path).await {
            Ok(content) => content,
            Err(err) => return Err(err.kind().into()),
        };
        Ok(Contents::from(content).into())
    }

    /// Checks if a file exists at the specified path within the disk-based
    /// storage.
    ///
    /// If the path does not point to a file, the method returns `Ok(false)`.
    /// Otherwise, it checks if the file exists and returns the result.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue checking the existence of the
    /// file.
    async fn file_exists(&self, path: &Path) -> DriverResult<bool> {
        if !path.is_file() {
            return Ok(false);
        }

        Ok(path.exists())
    }

    /// Writes the provided content to a file at the specified path within the
    /// disk-based storage.
    ///
    /// If the directory structure leading to the file does not exist, it
    /// creates the necessary directories.
    ///
    /// # Errors
    ///
    /// Returns an error if there is any issue creating directories, writing to
    /// the file, or handling other I/O-related errors.
    async fn write(&self, path: &Path, content: Vec<u8>) -> DriverResult<()> {
        let path = self.location.join(path);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if let Err(err) = fs::create_dir_all(parent).await {
                    return Err(err.kind().into());
                }
            }
        }

        match fs::write(path, content).await {
            Ok(()) => Ok(()),
            Err(err) => Err(err.kind().into()),
        }
    }

    /// Deletes the file at the specified path within the disk-based storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the file does not exist or if there is any issue
    /// deleting the file.
    ///
    /// If the file does not exist, the error variant
    /// `DriverError::ResourceNotFound` is returned.
    async fn delete(&self, path: &Path) -> DriverResult<()> {
        let path = self.location.join(path);
        if !path.exists() {
            return Err(DriverError::ResourceNotFound);
        };

        match fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(err) => Err(err.kind().into()),
        }
    }

    /// Deletes the directory and its contents at the specified path within the
    /// disk-based storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory does not exist or if there is any
    /// issue deleting the directory.
    ///
    /// If the directory does not exist, the error variant
    /// `DriverError::DirectoryNotFound` is returned.
    async fn delete_directory(&self, path: &Path) -> DriverResult<()> {
        let path = self.location.join(path);

        if !path.exists() {
            return Err(DriverError::ResourceNotFound);
        };

        match fs::remove_dir_all(path).await {
            Ok(()) => Ok(()),
            Err(err) => Err(err.kind().into()),
        }
    }

    /// Retrieves the last modification time of the file at the specified path
    /// within the disk-based storage. # Errors
    ///
    /// Returns an error if the file does not exist or if there is any issue
    /// retrieving the last modification time.
    ///
    /// If the file does not exist, the error variant
    /// `DriverError::ResourceNotFound` is returned.
    async fn last_modified(&self, path: &Path) -> DriverResult<SystemTime> {
        let path = self.location.join(path);
        if !path.exists() {
            return Err(DriverError::ResourceNotFound);
        }

        let metadata = match fs::metadata(path).await {
            Ok(metadata) => metadata,
            Err(err) => return Err(err.kind().into()),
        };

        match metadata.modified() {
            Ok(modified) => Ok(modified),
            Err(err) => Err(err.kind().into()),
        }
    }
}
