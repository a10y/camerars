use std::path::{Path, PathBuf};

use ffmpeg_next::{codec, Dictionary, encoder, format, Packet, Rational};
use ffmpeg_next::codec::Parameters;
use ffmpeg_next::format::context::Output;

use crate::chunk::ChunkWriter;

pub struct FileChunkWriter {
    path: PathBuf,
    ctx: Output,
}

impl FileChunkWriter {
    // Receive some sort of chunk ID here instead.
    pub fn new<P: AsRef<Path>>(path: &P) -> Self {
        println!("creating {:?}", path.as_ref());
        let ctx = format::output(path).expect("creating output context should succeed");
        let path = path.as_ref().into();

        Self {
            path,
            ctx,
        }
    }
}

impl ChunkWriter for FileChunkWriter {
    fn current_chunk_id(&self) -> usize {
        todo!()
    }

    fn begin(&mut self,
             metadata: Dictionary,
             video_parameters: Parameters, audio_parameters: Option<Parameters>) {
        // Initialize video stream
        println!("initializing video stream");
        let mut ost = self.ctx.add_stream(encoder::find(codec::Id::None))
            .expect("add_stream for video should succeed");
        ost.set_parameters(video_parameters);
        unsafe {
            (*ost.parameters().as_mut_ptr()).codec_tag = 0;
        }

        // Optionally, initialize audio stream
        if let Some(audio_parameters) = audio_parameters {
            println!("initializing audio stream");
            let mut ost = self.ctx.add_stream(encoder::find(codec::Id::None))
                .expect("add_stream for audio should succeed");
            ost.set_parameters(audio_parameters);
            unsafe {
                (*ost.parameters().as_mut_ptr()).codec_tag = 0;
            }
        }

        self.ctx.set_metadata(metadata);
        self.ctx.write_header().expect("writing output header should succeed");
    }

    fn video_timebase(&self) -> Rational {
        self.ctx.stream(0).expect("stream 0 should exist and be the video stream").time_base()
    }

    fn audio_timebase(&self) -> Option<Rational> {
        self.ctx.stream(1).map(|stream| stream.time_base())
    }

    fn write_video(&mut self, mut packet: Packet, src_timebase: Rational) {
        packet.rescale_ts(src_timebase, self.video_timebase());
        packet.set_position(-1);
        packet.set_stream(0);
        packet.write_interleaved(&mut self.ctx)
            .expect("expected write_interleaved for video to succeed");
    }

    fn write_audio(&mut self, mut packet: Packet, src_timebase: Rational) {
        packet.rescale_ts(src_timebase, self.audio_timebase().expect("audio_timebase should be non-None"));
        packet.set_position(-1);
        packet.set_stream(1);
        packet.write_interleaved(&mut self.ctx)
            .expect("expected write_interleaved for audio to succeed");
    }

    fn end(&mut self) {
        self.ctx.write_trailer().expect("writing output trailer should succeed");
    }
}
