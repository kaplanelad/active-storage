use crate::drivers::flow;
use active_storage::{
    drivers,
    drivers::{gcp_storage, gcp_storage::ClientBuilderTrait, Driver},
    StoreConfig,
};
use google_cloud_storage::http::{error::ErrorResponse, objects::Object, Error};
use std::{path::PathBuf, time::SystemTime};

const BUCKET_NAME: &str = "test-bucket";

#[derive(Clone)]
struct MockClient {
    inner: drivers::inmem::InMemoryDriver,
}

#[async_trait::async_trait]
impl ClientBuilderTrait for MockClient {
    async fn download_object(&self, bucket: &str, path: &str) -> Result<Vec<u8>, Error> {
        assert_eq!(bucket, BUCKET_NAME);
        let path = PathBuf::from(path);
        (self.inner.read(&path).await).map_or_else(
            |_| {
                let error_response = ErrorResponse {
                    errors: vec![],
                    message: "Not found".to_string(),
                    code: 404,
                };
                Err(Error::Response(error_response))
            },
            Ok,
        )
    }

    async fn get_object_details(&self, bucket: &str, path: &str) -> Result<Object, Error> {
        assert_eq!(bucket, BUCKET_NAME);
        let path = PathBuf::from(path);
        if self.inner.file_exists(path.as_path()).await.unwrap() {
            Ok(Self::create_default_object(path))
        } else {
            let error_response = ErrorResponse {
                errors: vec![],
                message: "Not found".to_string(),
                code: 404,
            };
            Err(Error::Response(error_response))
        }
    }

    async fn object_exists(&self, bucket: &str, path: &str) -> Result<bool, Error> {
        assert_eq!(bucket, BUCKET_NAME);
        let path = PathBuf::from(path);
        (self.inner.file_exists(path.as_path()).await).map_or_else(
            |_| {
                let error_response = ErrorResponse {
                    errors: vec![],
                    message: "Not found".to_string(),
                    code: 404,
                };
                Err(Error::Response(error_response))
            },
            Ok,
        )
    }

    async fn upload_objects(
        &self,
        bucket: &str,
        path: &str,
        content: Vec<u8>,
    ) -> Result<Object, Error> {
        assert_eq!(bucket, BUCKET_NAME);
        let path = PathBuf::from(path);
        let _ = self.inner.write(&path, content).await;
        let object = Self::create_default_object(path);
        Ok(object)
    }

    async fn delete_objects(&self, bucket: &str, path: &str) -> Result<(), Error> {
        assert_eq!(bucket, BUCKET_NAME);
        let path = PathBuf::from(path);
        if (self.inner.delete(path.as_path()).await).is_err() {
            let error_response = ErrorResponse {
                errors: vec![],
                message: "Not found".to_string(),
                code: 404,
            };
            Err(Error::Response(error_response))
        } else {
            Ok(())
        }
    }

    async fn list_objects(&self, bucket: &str, _path: &str) -> Result<Vec<PathBuf>, Error> {
        assert_eq!(bucket, BUCKET_NAME);
        let keys: Vec<PathBuf> = self.inner.files.lock().unwrap().keys().cloned().collect();
        Ok(keys)
    }
}

impl MockClient {
    fn create_default_object(path: PathBuf) -> Object {
        let object = Object {
            name: path.to_string_lossy().to_string(),
            bucket: BUCKET_NAME.to_string(),
            updated: Some(time::OffsetDateTime::from(SystemTime::now())),
            time_created: Some(time::OffsetDateTime::from(SystemTime::now())),
            ..Default::default()
        };
        object
    }
}

#[tokio::test]
async fn inmem() {
    let mock_client = Box::new(MockClient {
        inner: drivers::inmem::InMemoryDriver::default(),
    });
    let gcs_driver = Box::new(gcp_storage::GoogleCloudStorage::with_client(
        BUCKET_NAME,
        mock_client,
    )) as Box<dyn Driver>;

    let store = StoreConfig::with_driver(gcs_driver);

    flow::test_driver(&store, PathBuf::new()).await;
}
