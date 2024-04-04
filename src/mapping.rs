use std::sync::atomic::{AtomicU64, Ordering};

use ffmpeg_next::codec::Parameters;
use ffmpeg_next::format;
use ffmpeg_next::format::context::Input;
use ffmpeg_next::media::Type;

use crate::chunk::ChunkWriter;

pub struct Pipeline {
    input_context: Input,
    video_index: usize,
    audio_index: Option<usize>,
    video_parameters: Parameters,
    audio_parameters: Option<Parameters>,
    index_mapping: Vec<usize>,
}

impl Pipeline {
    /// Create a pipeline from an input stream (could be RTSP, file, MPEG-TS, etc.).
    /// Provide it with a chunk writer factory as well. So that way we can
    pub fn from<S: AsRef<str>>(url: S) -> Self {
        let input_context = format::input(&String::from(url.as_ref()))
            .expect("open input format");

        let video_index = input_context.streams()
            .best(Type::Video)
            .expect("expected input to contain at least one video stream")
            .index();

        let audio_index = input_context.streams()
            .best(Type::Audio)
            .map(|stream| stream.index());

        let video_parameters = input_context.stream(video_index).unwrap().parameters().to_owned();
        let audio_parameters = audio_index.map(|audio_index| {
            input_context.stream(audio_index).unwrap().parameters().to_owned()
        });

        // Each of these should matter here instead.
        // If we haven't done this, then we're screwed.
        let mut index_mapping = vec![usize::MAX; input_context.streams().len()];
        index_mapping[video_index] = 0;
        if let Some(audio_index) = audio_index {
            index_mapping[audio_index] = 1;
        }

        Self {
            input_context,
            video_index,
            audio_index,
            video_parameters,
            audio_parameters,
            index_mapping,
        }
    }

    pub fn run<C: ChunkWriter>(
        &mut self,
        chunk_writer: &mut C,
    ) {
        chunk_writer.begin(
            self.input_context.metadata().to_owned(),
            self.video_parameters.clone(), self.audio_parameters.clone());

        let video_packets = AtomicU64::new(0);
        let audio_packets = AtomicU64::new(0);
        let unknown_packets = AtomicU64::new(0);

        for (stream, packet) in self.input_context.packets() {
            let out_index = self.index_mapping[stream.index()];

            match out_index {
                _ if out_index == 0 => {
                    chunk_writer.write_video(packet, stream.time_base());
                    video_packets.fetch_add(1, Ordering::SeqCst);
                }
                _ if out_index == 1 && self.audio_index.is_some() => {
                    chunk_writer.write_audio(packet, stream.time_base());
                    audio_packets.fetch_add(1, Ordering::SeqCst);
                }

                _ => {
                    unknown_packets.fetch_add(1, Ordering::SeqCst);
                }
            }
        }

        chunk_writer.end();

        println!(
            "Processing Statistics: video={} audio={} other={}",
            video_packets.fetch_add(0, Ordering::Relaxed),
            audio_packets.fetch_add(0, Ordering::Relaxed),
            unknown_packets.fetch_add(0, Ordering::Relaxed),
        );
    }
}