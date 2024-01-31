use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Mutex,
    time::SystemTime,
};

use super::{Driver, DriverError};
use crate::{contents::Contents, errors::DriverResult};

#[derive(Debug, Clone)]
pub struct File {
    pub content: Vec<u8>,
    pub last_modified: SystemTime,
}

#[derive(Debug, Default)]
pub struct InMemoryDriver {
    files: Mutex<BTreeMap<PathBuf, File>>,
    directory: Mutex<BTreeMap<PathBuf, Vec<PathBuf>>>,
}

impl Clone for InMemoryDriver {
    fn clone(&self) -> Self {
        Self {
            files: Mutex::new(self.files.lock().unwrap().clone()),
            directory: Mutex::new(self.directory.lock().unwrap().clone()),
        }
    }
}

impl InMemoryDriver {
    fn get_files(&self) -> BTreeMap<PathBuf, File> {
        self.files
            .lock()
            .expect("inmem store failed getting a lock")
            .clone()
    }
}

#[async_trait::async_trait]
impl Driver for InMemoryDriver {
    async fn read(&self, path: &Path) -> DriverResult<Vec<u8>> {
        let files = self.get_files();
        let file = files.get(path).ok_or(DriverError::ResourceNotFound)?;

        Ok(Contents::from(file.content.clone()).into())
    }

    async fn file_exists(&self, path: &Path) -> DriverResult<bool> {
        Ok(self.get_files().contains_key(path))
    }

    async fn write(&self, path: &Path, content: Vec<u8>) -> DriverResult<()> {
        self.files.lock().unwrap().insert(
            path.to_path_buf(),
            File {
                last_modified: SystemTime::now(),
                content,
            },
        );

        if let Some(parent) = path.parent() {
            self.directory
                .lock()
                .unwrap()
                .entry(parent.to_path_buf())
                .or_default()
                .push(path.to_path_buf());
        }

        Ok(())
    }

    async fn delete(&self, path: &Path) -> DriverResult<()> {
        if self.files.lock().unwrap().remove(path).is_none() {
            return Err(DriverError::ResourceNotFound);
        }

        self.directory
            .lock()
            .unwrap()
            .entry(path.parent().unwrap().to_path_buf())
            .or_default()
            .retain(|file_path| file_path != path);

        Ok(())
    }

    async fn delete_directory(&self, path: &Path) -> DriverResult<()> {
        if !self.directory.lock().unwrap().contains_key(path) {
            return Err(DriverError::ResourceNotFound);
        }

        self.directory
            .lock()
            .unwrap()
            .retain(|file_path, _| !file_path.starts_with(path));

        self.files
            .lock()
            .unwrap()
            .retain(|file_path, _| !file_path.starts_with(path));

        Ok(())
    }

    async fn last_modified(&self, path: &Path) -> DriverResult<SystemTime> {
        let file = self.get_files();
        let file = file.get(path).ok_or(DriverError::ResourceNotFound)?;

        Ok(file.last_modified)
    }
}

#[cfg(test)]
mod tests {

    use insta::{assert_debug_snapshot, with_settings};
    use lazy_static::lazy_static;

    use super::*;

    lazy_static! {
        pub static ref CLEANUP_DATA: Vec<(&'static str, &'static str)> = vec![
            (r"tv_sec: (\d+),", "tv_sec: TV_SEC"),
            (r"tv_nsec: (\d+),", "tv_sec: TV_NSEC")
        ];
    }

    #[tokio::test]
    async fn validate_store() {
        let driver = InMemoryDriver::default();

        // cerate state
        let _ = driver
            .write(
                PathBuf::from("foo").join("file-1.txt").as_path(),
                b"".into(),
            )
            .await;

        let _ = driver
            .write(
                PathBuf::from("foo").join("file-2.txt").as_path(),
                b"".into(),
            )
            .await;

        let _ = driver
            .write(
                PathBuf::from("bar").join("file-1.txt").as_path(),
                b"".into(),
            )
            .await;
        let _ = driver
            .write(
                PathBuf::from("bar").join("file-2.txt").as_path(),
                b"".into(),
            )
            .await;

        // snapshot the state
        with_settings!({
            filters => CLEANUP_DATA.to_vec()
        }, {
        assert_debug_snapshot!(driver);
        });

        // delete folder
        assert!(driver
            .delete_directory(PathBuf::from("foo").as_path())
            .await
            .is_ok());

        // delete file
        assert!(driver
            .delete(PathBuf::from("bar").join("file-1.txt").as_path())
            .await
            .is_ok());

        with_settings!({
            filters => CLEANUP_DATA.to_vec()
        }, {
        assert_debug_snapshot!(driver);
        });
    }
}
