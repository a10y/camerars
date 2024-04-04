use std::ops::Mul;
use std::path::{Path, PathBuf};

use ffmpeg_next::{Dictionary, Packet, Rational};
use ffmpeg_next::codec::Parameters;

use crate::chunk::ChunkWriter;
use crate::chunk::file::FileChunkWriter;

/// A [ChunkWriter] that rolls chunks into files based on the time-length of each file.
/// Internally, we keep track of an incrementing counter and can process many chunks at once here.
/// The idea is that we always end up having a monotonically increasing clock speed from here too.
pub struct TimeBasedRollingChunkWriter {
    max_chunk_seconds: u32,
    directory: PathBuf,
    active_writer: FileChunkWriter,
    writer_num: usize,
    packets_written: usize,
    start_pts: Option<i64>,
    begin: Option<BeginParams>,
}

#[derive(Clone)]
struct BeginParams {
    metadata: Dictionary<'static>,
    video_params: Parameters,
    audio_params: Option<Parameters>,
}

fn writer_file<P: AsRef<Path>>(directory: &P, seq_num: usize) -> PathBuf {
    let file = format!("{:0>5}.ts", seq_num);
    directory.as_ref().join(file.as_str())
}

// TODO(aduffy): use SQLite to track the chunk IDs that should be offloaded.
impl TimeBasedRollingChunkWriter
{
    pub fn new<P: AsRef<Path>>(directory: &P, max_chunk_seconds: u32) -> Self {
        let directory = directory.as_ref().to_owned();
        let writer_num = 0;
        let active_file = writer_file(&directory, writer_num);
        let active_writer = FileChunkWriter::new(&active_file);

        Self {
            max_chunk_seconds,
            directory,
            active_writer,
            writer_num,
            packets_written: 0,
            start_pts: None,
            begin: None,
        }
    }

    fn should_roll(&mut self, pts: i64) -> bool {
        let start_pts = self.start_pts.get_or_insert(pts);
        let delta = pts - *start_pts;

        // Based on the time_base of the output (which should bt 90kHZ MPEG-TS) determine
        // if > `max_chunk_seconds` time has passed.
        Rational(delta as _, 1).mul(self.video_timebase()) > Rational(self.max_chunk_seconds as _, 1)
    }
}

impl ChunkWriter for TimeBasedRollingChunkWriter {
    fn current_chunk_id(&self) -> usize {
        todo!()
    }

    fn begin(&mut self, metadata: Dictionary<'static>, video_params: Parameters, audio_params: Option<Parameters>) {
        self.begin = Some(BeginParams {
            metadata: metadata.clone(),
            video_params: video_params.clone(),
            audio_params: audio_params.clone(),
        });
        self.active_writer.begin(metadata, video_params, audio_params);
    }

    fn video_timebase(&self) -> Rational {
        self.active_writer.video_timebase()
    }

    fn audio_timebase(&self) -> Option<Rational> {
        self.active_writer.audio_timebase()
    }

    fn write_video(&mut self, packet: Packet, src_timebase: Rational) {
        // TODO(aduffy): Add logging
        let pts = packet.pts().unwrap_or_default();
        if self.should_roll(pts) {
            self.start_pts = Some(pts);
            self.active_writer.end();

            // Construct new writer
            self.writer_num += 1;
            let active_file = writer_file(&self.directory, self.writer_num);
            self.active_writer = FileChunkWriter::new(&active_file);

            // Begin new writer
            let begin_params = self.begin.clone().expect("begin_params should be set");
            self.active_writer.begin(
                begin_params.metadata,
                begin_params.video_params,
                begin_params.audio_params,
            );
        }

        self.active_writer.write_video(packet, src_timebase);
        self.packets_written += 1;
    }

    fn write_audio(&mut self, packet: Packet, src_timebase: Rational) {
        self.active_writer.write_audio(packet, src_timebase);
    }

    fn end(&mut self) {
        // TODO(aduffy): we should make end() by a terminal state for all ChunkWriters, and
        //  consume the self.
        self.active_writer.end();
    }
}