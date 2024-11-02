extern crate ffmpeg_next as ffmpeg;

use std::sync::Arc;

use clap::Parser;
use dotenvy::dotenv_override;
use tracing::info;

use camerars::chunk::file::FileChunkWriterFactory;
use camerars::db::Database;
use camerars::execution::{Pipeline, PlaylistBuilder};
use camerars::server::make_server;
use camerars::upload::s3;

#[derive(Parser)]
pub struct Cli {
    pub source: String,
    #[clap(long)]
    pub prefix: Option<String>,
}

pub fn main() {
    tracing_subscriber::fmt::init();
    dotenv_override().ok();

    ffmpeg::init().expect("ffmpeg initialization should succeed");

    let cli = Cli::parse();

    let prefix = cli.prefix.unwrap_or_else(|| "/".to_string());

    let database = Database::file("v0.db");

    // Create a new runtime just for serving file requests from disk.
    let mut chunk_writer = FileChunkWriterFactory::new(&"recordings");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let uploader = s3::new_s3_uploader(prefix.as_ref(), runtime.handle().clone());
    let uploader = Arc::new(uploader);
    {
        let uploader = Arc::clone(&uploader);
        let database = database.clone();

        runtime.spawn(async move {
            let playlist_builder = PlaylistBuilder::new(&database);
            let service = make_server(playlist_builder, uploader);
            info!("Server is running @ 127.0.0.1:3030");

            warp::serve(service).run(([127, 0, 0, 1], 3030)).await
        });
    }

    Pipeline::from(cli.source.as_str())
        .with_roll_seconds(15)
        .run(&mut chunk_writer, Arc::clone(&uploader), &database);
}
