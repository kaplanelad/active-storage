//! # Active Storage
//!
//! Active Storage streamlines the process of uploading files to cloud storage,
//! offering both local disk-based and in-memory services for development and
//! testing. Additionally, it supports mirroring files to subordinate services,
//! enhancing capabilities for backups and migrations.
//!
//! It's inspired by Rails [Active Store](https://guides.rubyonrails.org/active_storage_overview.html)
//!
//! ## Services
//!
//! * [Disk](./examples/disk.rs)
//! * [In Memory](./examples/in_memory.rs)
//! * [AWS S3](./examples/aws_s3.rs) - Requires enabling the `aws_s3` feature.
//!
//! ## Examples
//!
//! ```rust
//! # #[cfg(feature = "derive")] {
#![doc = include_str!("../examples/disk.rs")]
//! # }
//! ```
//! 
//! ### Mirroring
//! ```rust
//! # #[cfg(feature = "derive")] {
#![doc = include_str!("../examples/multi.rs")]
//! # }
//! ```

mod contents;
pub mod drivers;
pub mod errors;
pub mod multi_store;
pub mod store;

/// The [`StoreConfig`] enum represents configuration options for building a
/// storage system. It includes different variants for various storage options,
/// and the availability of these variants depends on compile-time feature
/// flags.
///
/// ## Enum Variants
///
/// - `InMem`: In-memory storage variant. This variant is available when the
///   `inmem` feature is enabled.
///
/// - `AwsS3`: AWS S3 storage variant. This variant is available when the
///   `aws_s3` feature is enabled. It includes a configuration parameter.
///
/// - `Disk`: Disk storage variant. This variant is available when the `disk`
///   feature is enabled. It includes a configuration parameter.
///
/// - `Azure`: Azure storage variant. This variant is available when the `azure`
///   feature is enabled. It includes a configuration parameter.
pub enum StoreConfig {
    #[cfg(feature = "inmem")]
    InMem(),
    #[cfg(feature = "disk")]
    Disk(drivers::disk::Config),
    #[cfg(feature = "aws_s3")]
    AwsS3(drivers::aws_s3::Config),
    #[cfg(feature = "azure")]
    Azure(drivers::azure::Config),
    #[cfg(feature = "gcp_storage")]
    Gcp(drivers::gcp_storage::Config),
}

/// `StoreConfig` represents the configuration for creating a [`store::Store`]
/// instance.
impl StoreConfig {
    /// Builds a [`store::Store`] instance based on the configured storage type.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use active_storage::StoreConfig;
    ///
    /// async fn example() {
    ///     let inmem_driver = StoreConfig::InMem().build().await.unwrap();
    ///     let file_path = PathBuf::from("test.txt");
    ///     inmem_driver
    ///         .write(file_path.as_path(), "my content")
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    /// # Errors
    ///
    /// Returns a [`errors::DriverResult`] when could not initialize the driver
    /// store
    #[allow(clippy::unused_async)]
    pub async fn build(self) -> errors::DriverResult<store::Store> {
        let driver = match self {
            #[cfg(feature = "inmem")]
            Self::InMem() => {
                Box::<drivers::inmem::InMemoryDriver>::default() as Box<dyn drivers::Driver>
            }
            #[cfg(feature = "disk")]
            Self::Disk(config) => {
                Box::new(drivers::disk::DiskDriver::new(config).await?) as Box<dyn drivers::Driver>
            }
            #[cfg(feature = "aws_s3")]
            Self::AwsS3(config) => {
                Box::new(drivers::aws_s3::AwsS3::new(config)) as Box<dyn drivers::Driver>
            }
            #[cfg(feature = "azure")]
            Self::Azure(config) => {
                Box::new(drivers::azure::AzureDriver::new(config)) as Box<dyn drivers::Driver>
            }
            #[cfg(feature = "gcp_storage")]
            Self::Gcp(config) => {
                Box::new(drivers::gcp_storage::GoogleCloudStorage::new(config).await?) as Box<dyn drivers::Driver>
            }
        };

        Ok(store::Store::new(driver))
    }

    /// Creates a [`store::Store`] instance with the provided storage driver.
    #[must_use]
    pub fn with_driver(driver: Box<dyn drivers::Driver>) -> store::Store {
        store::Store::new(driver)
    }
}
