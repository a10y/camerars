extern crate ffmpeg_next as ffmpeg;

use std::env;
use std::sync::Arc;

use clap::Parser;
use dotenvy::dotenv_override;

use tracing::info;

use camerars::chunk::file::FileChunkWriterFactory;
use camerars::db::Database;
use camerars::execution::{Pipeline, PlaylistBuilder};
use camerars::server::make_server;
use camerars::upload::s3;
use camerars::upload::s3::S3UploaderConfig;

#[derive(Parser)]
pub struct Cli {
    pub source: String,
    pub bucket: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    #[clap(long)]
    pub prefix: Option<String>,
}

pub fn main() {
    tracing_subscriber::fmt::init();
    dotenv_override().unwrap();

    ffmpeg::init().expect("ffmpeg initialization should succeed");

    let cli = Cli::parse();

    let mut pipeline = Pipeline::from(cli.source.as_str()).with_roll_seconds(15);

    let bucket_name = cli
        .bucket
        .unwrap_or_else(|| env::var("AWS_BUCKET").unwrap());
    let aws_access_key_id = cli
        .access_key_id
        .unwrap_or_else(|| env::var("AWS_ACCESS_KEY_ID").unwrap());
    let aws_secret_access_key = cli
        .secret_access_key
        .unwrap_or_else(|| env::var("AWS_SECRET_ACCESS_KEY").unwrap());
    let prefix = cli.prefix.unwrap_or_else(|| "/".to_string());

    // Setup a DB
    let database = Database::file("v0.db");

    let db_server = database.clone();

    // Create a new runtime just for serving file requests from disk.
    let mut chunk_writer = FileChunkWriterFactory::new(&"recordings");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let uploader = s3::new_s3_uploader(
        &S3UploaderConfig {
            bucket: bucket_name,
            access_key_id: aws_access_key_id,
            secret_access_key: aws_secret_access_key,
        },
        &*prefix,
        runtime.handle().clone(),
    );
    let uploader = Arc::new(uploader);
    let server_uploader = Arc::clone(&uploader);

    runtime.spawn(async move {
        let playlist_builder = PlaylistBuilder::new(&db_server);
        let service = make_server(playlist_builder, server_uploader);
        info!("Server is running @ 127.0.0.1:3030");

        warp::serve(service).run(([127, 0, 0, 1], 3030)).await
    });

    pipeline.run(&mut chunk_writer, Arc::clone(&uploader), &database);
}
