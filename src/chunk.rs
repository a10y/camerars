use ffmpeg_next::codec::Parameters;
use ffmpeg_next::{Dictionary, Packet, Rational};

pub mod file;

mod private {
    pub trait Sealed {}

    // Impls
    impl Sealed for crate::chunk::file::FileChunkWriter {}
}

pub trait ChunkWriter: private::Sealed {
    fn begin(
        &mut self,
        metadata: &Dictionary<'static>,
        video_params: Parameters,
        audio_params: Option<Parameters>,
    );

    fn video_timebase(&self) -> Rational;

    fn audio_timebase(&self) -> Option<Rational>;

    /// Write another frame into this chunk.
    fn write_video(&mut self, packet: Packet, src_timebase: Rational);

    fn write_audio(&mut self, packet: Packet, src_timebase: Rational);

    /// Close any resources associated with this chunk.
    fn end(&mut self) -> std::path::PathBuf;
}

pub trait ChunkWriterFactory {
    type Target: ChunkWriter;

    fn init(&mut self);

    fn next(&mut self) -> Self::Target;
}
