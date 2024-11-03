use std::sync::Arc;

use bytes::Bytes;
use object_store::path::Path;
use object_store::ObjectStore;
use tracing::info;

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

pub fn new_s3_uploader(prefix: impl Into<Path>) -> ObjectStoreUploader {
    let object_store = object_store::aws::AmazonS3Builder::from_env()
        .build()
        .expect("expected S3 client to build");

    ObjectStoreUploader::new(Arc::new(object_store), prefix)
}

impl ObjectStoreUploader {
    pub fn new<P: Into<Path>>(object_store: Arc<dyn ObjectStore>, prefix: P) -> Self {
        Self {
            object_store: Arc::clone(&object_store),
            prefix: prefix.into(),
        }
    }
}

impl Uploader for ObjectStoreUploader {
    async fn upload_chunk(&self, name: &str, chunk: Vec<u8>) -> anyhow::Result<()> {
        info!(name = name, "Uploading chunk to remote store");

        let target_path = self.prefix.clone().child(name);

        s3_upload_chunk(target_path, chunk, self.object_store.clone()).await
    }

    async fn read_chunk(&self, name: &str) -> Vec<u8> {
        info!(name = name, "Reading chunk from remote storage");
        let target_path = self.prefix.clone().child(name);

        self.object_store
            .clone()
            .get(&target_path)
            .await
            .expect("object_store GET should succeed")
            .bytes()
            .await
            .unwrap()
            .to_vec()
    }
}

async fn s3_upload_chunk(
    target: Path,
    data: Vec<u8>,
    store: Arc<dyn ObjectStore>,
) -> anyhow::Result<()> {
    let bytes = Bytes::from(data);
    store.put(&target, bytes).await?;
    Ok(())
}
