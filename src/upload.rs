use std::io::Read;

pub mod s3;

/// Uploader indicates which uploaders are available, if possible.
/// We want to support a distributed instance of the Slice to get a list of all available
/// UploaderFactory instances.
pub trait Uploader: Send + Sync + Clone {
    fn upload_chunk<R: Read>(&self, name: &str, chunk: R);
    // Return back a buffer, potentially with a range request to return a set of bytes.
    fn read_chunk(&self, name: &str) -> Vec<u8>;
}
