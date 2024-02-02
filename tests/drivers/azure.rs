use std::{path::PathBuf, time::SystemTime};

use active_storage::{
    drivers::{self, azure, Driver},
    StoreConfig,
};
use azure_storage_blobs::blob::operations::DeleteBlobResponse;

use super::flow;

const CONTAINER_NAME: &str = "test-container";

#[derive(Clone)]
struct MockClient {
    inner: drivers::inmem::InMemoryDriver,
}

#[async_trait::async_trait]
impl azure::ClientBuilderTrait for MockClient {
    async fn get_blob_content(&self, container: &str, path: &str) -> azure_core::Result<Vec<u8>> {
        assert_eq!(container, CONTAINER_NAME);
        let path: PathBuf = PathBuf::from(path);

        (self.inner.read(&path).await).map_or_else(
            |_| {
                let kind = azure_storage::ErrorKind::HttpResponse {
                    status: azure_core::StatusCode::NotFound,
                    error_code: Some("BlobNotFound".to_string()),
                };

                Err(azure_core::error::Error::message(kind, ""))
            },
            Ok,
        )
    }

    async fn blob_exists(&self, container: &str, path: &str) -> azure_core::Result<bool> {
        assert_eq!(container, CONTAINER_NAME);
        let path = PathBuf::from(path);

        (self.inner.file_exists(path.as_path()).await).map_or_else(
            |_| {
                let kind = azure_storage::ErrorKind::HttpResponse {
                    status: azure_core::StatusCode::NotFound,
                    error_code: Some("BlobNotFound".to_string()),
                };

                Err(azure_core::error::Error::message(kind, ""))
            },
            Ok,
        )
    }

    async fn put_block_blob(
        &self,
        container: &str,
        path: &str,
        content: Vec<u8>,
    ) -> azure_core::Result<()> {
        assert_eq!(container, CONTAINER_NAME);

        let path: PathBuf = PathBuf::from(path);
        let _ = self.inner.write(&path, content).await;
        Ok(())
    }

    async fn delete(&self, container: &str, path: &str) -> azure_core::Result<DeleteBlobResponse> {
        assert_eq!(container, CONTAINER_NAME);
        let path = PathBuf::from(path);

        if (self.inner.delete(path.as_path()).await).is_err() {
            let kind = azure_storage::ErrorKind::HttpResponse {
                status: azure_core::StatusCode::NotFound,
                error_code: Some("BlobNotFound".to_string()),
            };

            Err(azure_core::error::Error::message(kind, ""))
        } else {
            Ok(DeleteBlobResponse {
                delete_type_permanent: false,
                request_id: azure_core::RequestId::new_v4(),
                date: SystemTime::now().into(),
            })
        }

        // Ok(res)
    }

    async fn get_properties(
        &self,
        container: &str,
        path: &str,
    ) -> azure_core::Result<azure::BlobProperties> {
        assert_eq!(container, CONTAINER_NAME);
        let path = PathBuf::from(path);

        if self.inner.file_exists(path.as_path()).await.unwrap() {
            Ok(azure::BlobProperties {
                date: SystemTime::now(),
            })
        } else {
            let kind = azure_storage::ErrorKind::HttpResponse {
                status: azure_core::StatusCode::NotFound,
                error_code: Some("BlobNotFound".to_string()),
            };

            Err(azure_core::error::Error::message(kind, ""))
        }
    }

    async fn list_blobs(&self, container: &str) -> azure_core::Result<Vec<PathBuf>> {
        assert_eq!(container, CONTAINER_NAME);
        let keys: Vec<PathBuf> = self.inner.files.lock().unwrap().keys().cloned().collect();
        Ok(keys)
    }
}

#[tokio::test]
async fn inmem() {
    let mock_client = Box::new(MockClient {
        inner: drivers::inmem::InMemoryDriver::default(),
    });
    let azure_driver =
        Box::new(azure::AzureDriver::with_client(CONTAINER_NAME, mock_client)) as Box<dyn Driver>;

    let store = StoreConfig::with_driver(azure_driver);

    flow::test_driver(&store, PathBuf::new()).await;
}
