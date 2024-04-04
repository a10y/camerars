pub mod s3;

mod private {
    pub trait Sealed {}
}

/// Uploader indicates which uploaders are available, if possible.
/// We want to support a distributed instance of the Slice to get a list of all available
/// UploaderFactory instances.
pub trait Uploader {
    // Return a thing that does synchronous uploads.
    // Once we've done that, it should be gtg.
    // We can push a new ChunkRef -> gives us read-only access to a single element on the DB.
}

// We don't really even need an uploader, do we.
// We can just do everything via transcoding.
