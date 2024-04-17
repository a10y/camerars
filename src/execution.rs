use std::fs::File;
use std::ops::Mul;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

use chrono::Utc;
use ffmpeg_next::codec::Parameters;
use ffmpeg_next::format::context::Input;
use ffmpeg_next::media::Type;
use ffmpeg_next::{format, Rational};
use tracing::info;

use crate::chunk::{ChunkWriter, ChunkWriterFactory};
use crate::db::Database;
use crate::playlist::{OnDemandTimeRange, Playlist, PlaylistFile, PlaylistKind};
use crate::upload::Uploader;

pub struct Pipeline {
    input_context: Input,
    audio_index: Option<usize>,
    video_parameters: Parameters,
    audio_parameters: Option<Parameters>,
    index_mapping: Vec<usize>,
    roll_seconds: u32,
}

impl Pipeline {
    /// Create a pipeline from an input stream (could be RTSP, file, MPEG-TS, etc.).
    /// Provide it with a chunk writer factory as well. So that way we can
    pub fn from<S: AsRef<str>>(url: S) -> Self {
        let input_context = format::input(&String::from(url.as_ref())).expect("open input format");

        let video_index = input_context
            .streams()
            .best(Type::Video)
            .expect("expected input to contain at least one video stream")
            .index();

        let audio_index = input_context
            .streams()
            .best(Type::Audio)
            .map(|stream| stream.index());

        let video_parameters = input_context
            .stream(video_index)
            .unwrap()
            .parameters()
            .to_owned();
        let audio_parameters = audio_index.map(|audio_index| {
            input_context
                .stream(audio_index)
                .unwrap()
                .parameters()
                .to_owned()
        });

        let mut index_mapping = vec![usize::MAX; input_context.streams().len()];
        index_mapping[video_index] = 0;
        if let Some(audio_index) = audio_index {
            index_mapping[audio_index] = 1;
        }

        Self {
            input_context,
            audio_index,
            video_parameters,
            audio_parameters,
            index_mapping,
            roll_seconds: 10,
        }
    }

    pub fn with_roll_seconds(mut self, new_roll_seconds: u32) -> Self {
        self.roll_seconds = new_roll_seconds;

        self
    }

    pub fn run<F: ChunkWriterFactory, U: Uploader + 'static>(
        &mut self,
        chunk_writers: &mut F,
        chunk_uploader: Arc<U>,
        database: &Database,
    ) {
        info!("begin pipeline");
        let mut chunk_writer = chunk_writers.next();
        let metadata = self.input_context.metadata().to_owned().clone();
        chunk_writer.begin(
            &metadata,
            self.video_parameters.clone(),
            self.audio_parameters.clone(),
        );

        let video_packets = AtomicU64::new(0);
        let audio_packets = AtomicU64::new(0);
        let unknown_packets = AtomicU64::new(0);

        let (tx, rx) = channel::<PathBuf>();
        let chunk_uploader = Arc::clone(&chunk_uploader);
        std::thread::spawn(move || {
            do_upload(chunk_uploader, rx);
        });

        let mut current_chunk_start = Utc::now();

        let mut start_pts = -1;
        for (stream, packet) in self.input_context.packets() {
            let out_index = self.index_mapping[stream.index()];

            let should_roll = match out_index {
                _ if out_index == 0 => {
                    let pts = packet.pts().unwrap_or_default();
                    chunk_writer.write_video(packet, stream.time_base());
                    video_packets.fetch_add(1, Ordering::SeqCst);

                    if start_pts < 0 {
                        start_pts = pts;
                        false
                    } else {
                        // check if we should roll
                        should_roll(start_pts, pts, stream.time_base(), self.roll_seconds)
                    }
                }
                _ if out_index == 1 && self.audio_index.is_some() => {
                    chunk_writer.write_audio(packet, stream.time_base());
                    audio_packets.fetch_add(1, Ordering::SeqCst);
                    false
                }
                _ => {
                    unknown_packets.fetch_add(1, Ordering::SeqCst);
                    false
                }
            };

            if should_roll {
                info!("rolling output file");
                let file_path = chunk_writer.end();
                start_pts = -1;

                // Update DB with new file
                database.append_file(
                    current_chunk_start,
                    PlaylistFile {
                        duration: 15.16, // TODO(aduffy) this is hard-coded, bad.
                        id: file_path.file_name().unwrap().to_str().unwrap().to_string(),
                    },
                );

                tx.send(file_path).unwrap();

                chunk_writer = chunk_writers.next();
                chunk_writer.begin(
                    &metadata,
                    self.video_parameters.clone(),
                    self.audio_parameters.clone(),
                );
                current_chunk_start = Utc::now();
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

fn should_roll(start_pts: i64, current_pts: i64, time_base: Rational, roll_seconds: u32) -> bool {
    let delta = current_pts - start_pts;

    Rational(delta as _, 1).mul(time_base) >= Rational(roll_seconds as _, 1)
}

fn do_upload<U: Uploader + 'static>(chunk_uploader: Arc<U>, rx: Receiver<PathBuf>) {
    while let Ok(msg) = rx.recv() {
        let name = msg.file_name().unwrap().to_str().unwrap();
        let file = File::open(&msg).unwrap();
        chunk_uploader.upload_chunk(name, &file);
    }
}

#[derive(Clone)]
pub struct PlaylistBuilder {
    db: Database,
}

impl PlaylistBuilder {
    pub fn new(db: &Database) -> Self {
        Self { db: db.clone() }
    }
}

impl PlaylistBuilder {
    pub fn build_on_demand(&self, time_range: OnDemandTimeRange) -> Playlist {
        let files = self
            .db
            .query_files(Some(time_range.start), Some(time_range.end));

        Playlist {
            kind: PlaylistKind::VOD,
            files,
        }
    }
}
