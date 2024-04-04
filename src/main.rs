extern crate ffmpeg_next as ffmpeg;

use clap::Parser;

use camerars::chunk::time::TimeBasedRollingChunkWriter;
use camerars::mapping::Pipeline;

#[derive(Parser)]
pub struct Cli {
    pub source: String,
}

pub fn main() {
    ffmpeg::init().expect("ffmpeg initialization should succeed");

    let cli = Cli::parse();

    let mut pipeline = Pipeline::from(cli.source.as_str());
    let mut chunk_writer = TimeBasedRollingChunkWriter::new(
        &"recordings",
        10,
    );
    pipeline.run(&mut chunk_writer);
}
