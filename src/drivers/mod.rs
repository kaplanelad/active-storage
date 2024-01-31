//! # Storage Driver Module
//!
//! The `storage_driver` module defines a trait `Driver` that represents a
//! storage driver, providing methods.
//!
use std::{path::Path, time::SystemTime};

use dyn_clone::DynClone;

use crate::errors::{DriverError, DriverResult};

#[cfg(feature = "disk")]
pub mod disk;

#[cfg(feature = "inmem")]
pub mod inmem;

#[cfg(feature = "aws_s3")]
pub mod aws_s3;

#[async_trait::async_trait]
pub trait Driver: DynClone + Sync + Send {
    async fn read(&self, path: &Path) -> DriverResult<Vec<u8>>;

    async fn file_exists(&self, path: &Path) -> DriverResult<bool>;

    async fn write(&self, path: &Path, content: Vec<u8>) -> DriverResult<()>;

    async fn delete(&self, path: &Path) -> DriverResult<()>;

    async fn delete_directory(&self, path: &Path) -> DriverResult<()>;

    async fn last_modified(&self, path: &Path) -> DriverResult<SystemTime>;
}
