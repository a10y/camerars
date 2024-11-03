use std::future::Future;

pub mod s3;

/// Uploader indicates which uploaders are available, if possible.
/// We want to support a distributed instance of the Slice to get a list of all available
/// UploaderFactory instances.
pub trait Uploader: Send + Sync + Clone {
    fn upload_chunk(
        &self,
        name: &str,
        chunk: Vec<u8>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    // Return back a buffer, potentially with a range request to return a set of bytes.
    fn read_chunk(&self, name: &str) -> impl Future<Output = Vec<u8>> + Send;
}
