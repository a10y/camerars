extern crate ffmpeg_next as ffmpeg;

use std::env;
use std::sync::Arc;

use clap::Parser;
use dotenvy::dotenv_override;
use object_store::aws::AmazonS3Builder;

use camerars::chunk::file::FileChunkWriterFactory;
use camerars::execution::Pipeline;
use camerars::upload::s3::ObjectStoreUploader;

#[derive(Parser)]
pub struct Cli {
    pub source: String,
    pub bucket: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
}

pub fn main() {
    tracing_subscriber::fmt::init();
    dotenv_override().unwrap();

    ffmpeg::init().expect("ffmpeg initialization should succeed");

    let cli = Cli::parse();

    let mut pipeline = Pipeline::from(cli.source.as_str())
        .with_roll_seconds(15);

    let bucket_name = cli.bucket
        .unwrap_or_else(|| env::var("AWS_BUCKET").unwrap());
    let aws_access_key_id = cli.access_key_id
        .unwrap_or_else(|| env::var("AWS_ACCESS_KEY_ID").unwrap());
    let aws_secret_access_key = cli.secret_access_key
        .unwrap_or_else(|| env::var("AWS_SECRET_ACCESS_KEY").unwrap());


    let mut chunk_writer = FileChunkWriterFactory::new(&"recordings");
    let uploader = ObjectStoreUploader::new(
        Arc::new(AmazonS3Builder::new()
            .with_bucket_name(bucket_name)
            .with_access_key_id(aws_access_key_id)
            .with_secret_access_key(aws_secret_access_key)
            .build()
            .unwrap()),
        "/",
    );
    pipeline.run(&mut chunk_writer, uploader);
}
