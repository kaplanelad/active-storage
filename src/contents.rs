//! # Contents Module
//!
//! The [`Contents`] module provides a simple struct to hold byte data
//! conversions.

/// The `Contents` struct represents a container for byte data.
pub struct Contents {
    data: Vec<u8>,
}

impl Contents {
    #[cfg(feature = "aws_s3")]
    /// Creates a new `Contents` instance from a byte stream.
    ///
    /// This function is available only when the "`aws_s3`" feature is enabled.
    /// # Returns
    ///
    /// Returns a `Result` containing a `Contents` instance with the collected
    /// byte data, or an error if the byte stream collection fails.
    pub(crate) async fn from_bytestream(
        bytes: aws_smithy_types::byte_stream::ByteStream,
    ) -> Result<Self, aws_smithy_types::byte_stream::error::Error> {
        Ok(Self {
            data: bytes.collect().await?.to_vec(),
        })
    }
}

impl From<Contents> for Vec<u8> {
    /// Converts a `Contents` instance into a `Vec<u8>`.
    ///
    /// # Returns
    ///
    /// Returns a `Vec<u8>` containing the byte data from the `Contents`
    /// instance.
    fn from(contents: Contents) -> Self {
        contents.data
    }
}

impl From<Vec<u8>> for Contents {
    /// Converts a `Vec<u8>` into a `Contents` instance.
    ///
    /// # Returns
    ///
    /// Returns a `Contents` instance with the provided byte data.
    fn from(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl TryFrom<Contents> for String {
    type Error = std::string::FromUtf8Error;

    /// Tries to convert a `Contents` instance into a `String`.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `String` with the UTF-8 representation
    /// of the byte data, or an error if the conversion fails.
    fn try_from(contents: Contents) -> Result<Self, Self::Error> {
        Self::from_utf8(contents.data)
    }
}
