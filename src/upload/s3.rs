use std::io::{Read};
use std::ops::Deref;
use std::sync::Arc;

use bytes::Bytes;
use object_store::ObjectStore;
use tracing::{info, instrument};

use crate::upload::Uploader;

#[derive(Clone)]
pub struct ObjectStoreUploader {
    object_store: Arc<dyn ObjectStore>,
    prefix: object_store::path::Path,
}

pub struct S3UploaderConfig {
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

pub struct Path(object_store::path::Path);

impl From<Path> for object_store::path::Path {
    fn from(value: Path) -> Self {
        value.0
    }
}

impl From<object_store::path::Path> for Path {
    fn from(value: object_store::path::Path) -> Self {
        Path(value)
    }
}

impl From<&str> for Path {
    fn from(value: &str) -> Self {
        Path(object_store::path::Path::from(value))
    }
}

impl Deref for Path {
    type Target = object_store::path::Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn new_s3_uploader(credentials: &S3UploaderConfig, prefix: impl Into<Path>) -> ObjectStoreUploader {
    let object_store = object_store::aws::AmazonS3Builder::new()
        .with_access_key_id(&credentials.access_key_id)
        .with_secret_access_key(&credentials.secret_access_key)
        .with_bucket_name(&credentials.bucket)
        .build()
        .expect("expected S3 client to build");

    ObjectStoreUploader::new(Arc::new(object_store), prefix)
}

impl ObjectStoreUploader {
    pub fn new<P: Into<Path>>(object_store: Arc<dyn ObjectStore>, prefix: P) -> Self {
        Self {
            object_store: Arc::clone(&object_store),
            prefix: prefix.into().into(),
        }
    }
}

unsafe impl Send for ObjectStoreUploader {}

unsafe impl Sync for ObjectStoreUploader {}

impl Uploader for ObjectStoreUploader {
    #[instrument(skip_all)]
    fn upload_chunk<R: Read>(&self, name: &str, mut chunk: R) {
        info!(name = name, "Uploading chunk to remote store");

        let target_path = object_store::path::Path::from(self.prefix.clone())
            .child(name);
        let mut vec = Vec::new();
        chunk.read_to_end(&mut vec).expect("expected chunk read to succeed");
        let bytes = Bytes::from(vec);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("expected current_thread executor to build");

        let _ = rt.block_on(async move {
            self.object_store.put(&target_path, bytes).await
        }).expect("object_store put should succeed");
    }
}
