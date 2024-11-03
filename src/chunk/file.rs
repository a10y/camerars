use std::path::{Path, PathBuf};

use ffmpeg_next::codec::Parameters;
use ffmpeg_next::format::context::Output;
use ffmpeg_next::{codec, encoder, format, Dictionary, Packet, Rational};
use tracing::{debug, info};

use crate::chunk::{ChunkWriter, ChunkWriterFactory};

pub struct FileChunkWriterFactory {
    directory: PathBuf,
    seq_num: u64,
}

impl FileChunkWriterFactory {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            seq_num: 0,
            directory: path.as_ref().to_path_buf(),
        }
    }
}

impl ChunkWriterFactory for FileChunkWriterFactory {
    type Target = FileChunkWriter;

    fn init(&mut self) {
        // create the directory.
        std::fs::create_dir_all(&self.directory).ok();

        // Find the last sequence number
        self.seq_num = std::fs::read_dir(&self.directory)
            .expect("directory created in init")
            .filter_map(|dirent| {
                let dirent = dirent.expect("dirent readable");
                if !dirent.file_type().expect("file_type").is_file() {
                    None
                } else {
                    dirent
                        .file_name()
                        .to_string_lossy()
                        .strip_suffix(".ts")
                        .and_then(|num| num.parse().ok())
                }
            })
            .max()
            .unwrap_or_default();

        info!("initializing with seq_num {}", self.seq_num);
    }

    fn next(&mut self) -> Self::Target {
        self.seq_num += 1;

        FileChunkWriter::new(
            &self
                .directory
                .clone()
                .join(format!("{:0>9}.ts", self.seq_num))
                .to_path_buf(),
        )
    }
}

pub struct FileChunkWriter {
    ctx: Output,
    path: PathBuf,
}

impl FileChunkWriter {
    pub fn new<P: AsRef<Path>>(path: &P) -> Self {
        let ctx = format::output(path).expect("creating output context should succeed");
        let path = path.as_ref().into();

        Self { ctx, path }
    }
}

impl ChunkWriter for FileChunkWriter {
    fn begin(
        &mut self,
        metadata: &Dictionary,
        video_parameters: Parameters,
        audio_parameters: Option<Parameters>,
    ) {
        debug!("initializing video stream");
        let mut ost = self
            .ctx
            .add_stream(encoder::find(codec::Id::None))
            .expect("add_stream for video should succeed");
        ost.set_parameters(video_parameters);
        unsafe {
            (*ost.parameters().as_mut_ptr()).codec_tag = 0;
        }

        // Optionally, initialize audio stream
        if let Some(audio_parameters) = audio_parameters {
            debug!("initializing audio stream");
            let mut ost = self
                .ctx
                .add_stream(encoder::find(codec::Id::None))
                .expect("add_stream for audio should succeed");
            ost.set_parameters(audio_parameters);
            unsafe {
                (*ost.parameters().as_mut_ptr()).codec_tag = 0;
            }
        }

        self.ctx.set_metadata(metadata.clone());
        self.ctx
            .write_header()
            .expect("writing output header should succeed");
    }

    fn video_timebase(&self) -> Rational {
        self.ctx
            .stream(0)
            .expect("stream 0 should exist and be the video stream")
            .time_base()
    }

    fn audio_timebase(&self) -> Option<Rational> {
        self.ctx.stream(1).map(|stream| stream.time_base())
    }

    fn write_video(&mut self, mut packet: Packet, src_timebase: Rational) {
        packet.rescale_ts(src_timebase, self.video_timebase());
        packet.set_position(-1);
        packet.set_stream(0);
        packet
            .write_interleaved(&mut self.ctx)
            .expect("expected write_interleaved for video to succeed");
    }

    fn write_audio(&mut self, mut packet: Packet, src_timebase: Rational) {
        packet.rescale_ts(
            src_timebase,
            self.audio_timebase()
                .expect("audio_timebase should be non-None"),
        );
        packet.set_position(-1);
        packet.set_stream(1);
        packet
            .write_interleaved(&mut self.ctx)
            .expect("expected write_interleaved for audio to succeed");
    }

    fn end(&mut self) -> PathBuf {
        self.ctx
            .write_trailer()
            .expect("writing output trailer should succeed");

        self.path.clone()
    }
}
