use ffmpeg_next::format::context::Input;
use ffmpeg_next::format::input;
use ffmpeg_next::{media, Stream};
use std::path::Path;

pub enum TimedPacket {
    Video(ffmpeg_next::Packet, f32),
    #[allow(unused)] // This variant is unused but kept for future use
    Audio(ffmpeg_next::Packet, f32),
}

pub struct Demuxer {
    input_context: Input,
    video_stream_idx: Option<usize>,
    audio_stream_idx: Option<usize>,
}

impl Demuxer {
    pub fn new(path: &Path) -> Result<Self, ffmpeg_next::Error> {
        let input_context = input(path)?;

        let video_stream = input_context.streams().best(media::Type::Video);
        let audio_stream = input_context.streams().best(media::Type::Audio);

        let video_stream_idx = video_stream.map(|s| s.index());
        let audio_stream_idx = audio_stream.map(|s| s.index());

        if video_stream_idx.is_none() && audio_stream_idx.is_none() {
            return Err(ffmpeg_next::Error::StreamNotFound);
        }

        Ok(Self {
            input_context,
            video_stream_idx,
            audio_stream_idx,
        })
    }

    pub fn read(&mut self) -> Option<TimedPacket> {
        let mut error_count = 0;

        loop {
            match self.input_context.packets().next() {
                Some((stream, packet)) => {
                    let time = packet.pts().map(|time| (time as f32) * (stream.time_base().numerator() as f32 / stream.time_base().denominator() as f32)).unwrap_or(0.0);
                    if let Some(video_idx) = self.video_stream_idx
                        && stream.index() == video_idx
                    {
                        return Some(TimedPacket::Video(
                            packet,
                            time,
                        ));
                    } else if let Some(audio_idx) = self.audio_stream_idx
                        && stream.index() == audio_idx
                    {
                        return Some(TimedPacket::Audio(
                            packet,
                            time,
                        ));
                    }
                }
                None => {
                    error_count += 1;
                    if error_count > 3 {
                        return None;
                    }
                }
            }
        }
    }

    pub fn seek_to_start(&mut self) -> Result<(), ffmpeg_next::Error> {
        self.input_context.seek(i64::MIN, ..)
    }

    pub fn video_stream(&self) -> Option<Stream> {
        self.video_stream_idx
            .map(|stream_idx| self.input_context.stream(stream_idx).unwrap())
    }
}
