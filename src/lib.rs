//! # Store Module
//!
//! The [`store`] module provides a flexible storage abstraction with
//! configurable drivers.
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
///   `aws_s3` feature is enabled. It includes a configuration parameter of type
///   [`drivers::aws_s3::Config`].
///
/// - `Disk`: Disk storage variant. This variant is available when the `disk`
///   feature is enabled. It includes a configuration parameter of type
///   [`drivers::disk::Config`].
pub enum StoreConfig {
    #[cfg(feature = "inmem")]
    InMem(),
    #[cfg(feature = "aws_s3")]
    AwsS3(drivers::aws_s3::Config),
    #[cfg(feature = "disk")]
    Disk(drivers::disk::Config),
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
            #[cfg(feature = "aws_s3")]
            Self::AwsS3(config) => {
                Box::new(drivers::aws_s3::AwsS3::new(config)) as Box<dyn drivers::Driver>
            }
            #[cfg(feature = "disk")]
            Self::Disk(config) => {
                Box::new(drivers::disk::DiskDriver::new(config).await?) as Box<dyn drivers::Driver>
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
