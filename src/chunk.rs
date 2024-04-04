use ffmpeg_next::codec::Parameters;
use ffmpeg_next::{Dictionary, Packet, Rational};

pub mod file;
pub mod time;

mod private {
    pub trait Sealed {}


    // Impls
    impl Sealed for crate::chunk::file::FileChunkWriter {}

    impl Sealed for crate::chunk::time::TimeBasedRollingChunkWriter {}
}


pub trait ChunkWriter: private::Sealed {
    fn current_chunk_id(&self) -> usize;

    fn begin(&mut self,
             metadata: Dictionary<'static>,
             video_params: Parameters,
             audio_params: Option<Parameters>);

    fn video_timebase(&self) -> Rational;

    fn audio_timebase(&self) -> Option<Rational>;

    /// Write another frame into this chunk.
    fn write_video(&mut self, packet: Packet, src_timebase: Rational);

    fn write_audio(&mut self, packet: Packet, src_timebase: Rational);

    /// Close any resources associated with this chunk.
    fn end(&mut self);
}

// Every chunk needs to have some sort of unique ID. This includes
// which stream it was a part of, and some sort of monotonically increasing string (assuming
// the clock is monotonic).