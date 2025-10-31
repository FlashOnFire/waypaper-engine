use ffmpeg_next::Dictionary;
use ffmpeg_next::format::context::Input;
use ffmpeg_next::format::input_with_dictionary;
use ffmpeg_next::{Stream, media};
use std::path::Path;

pub enum Packet {
    Video(ffmpeg_next::Packet),
    #[allow(unused)] // This variant is unused but kept for future use
    Audio(ffmpeg_next::Packet),
}

pub struct Demuxer {
    input_context: Input,
    video_stream_idx: Option<usize>,
    audio_stream_idx: Option<usize>,
}

impl Demuxer {
    pub fn new(path: &Path) -> Result<Self, ffmpeg_next::Error> {
        let mut opts = Dictionary::new();
        opts.set("genpts", "1");
        let mut input_context = input_with_dictionary(path, opts)?;
        let video_stream = input_context.streams().best(media::Type::Video);
        let audio_stream = input_context.streams().best(media::Type::Audio);

        let video_stream_idx = video_stream.as_ref().map(|s| s.index());
        let audio_stream_idx = audio_stream.as_ref().map(|s| s.index());

        if let Some(video_stream) = video_stream
            && video_stream.time_base().numerator() == 0
        {
            tracing::warn!(
                "Video stream time base is zero. This will cause issues. Trying to set it based on average frame rate..."
            );
            let new_time_base = video_stream.avg_frame_rate().invert();
            tracing::warn!(
                "Setting video stream time base to {}/{}",
                new_time_base.numerator(),
                new_time_base.denominator()
            );

            input_context
                .stream_mut(video_stream_idx.unwrap())
                .unwrap()
                .set_time_base(new_time_base);
        }

        if video_stream_idx.is_none() && audio_stream_idx.is_none() {
            return Err(ffmpeg_next::Error::StreamNotFound);
        }

        Ok(Self {
            input_context,
            video_stream_idx,
            audio_stream_idx,
        })
    }

    pub fn read(&mut self) -> Option<Packet> {
        let mut error_count = 0;

        loop {
            match self.input_context.packets().next() {
                Some((stream, mut packet)) => {
                    if packet.time_base().numerator() == 0 {
                        tracing::debug!(
                            "Packet time base is zero. This will cause issues. Setting it to stream time base"
                        );
                        packet.set_time_base(stream.time_base());
                    };

                    if let Some(video_idx) = self.video_stream_idx
                        && stream.index() == video_idx
                    {
                        return Some(Packet::Video(packet));
                    } else if let Some(audio_idx) = self.audio_stream_idx
                        && stream.index() == audio_idx
                    {
                        return Some(Packet::Audio(packet));
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

    pub fn video_stream(&self) -> Option<Stream<'_>> {
        self.video_stream_idx
            .map(|stream_idx| self.input_context.stream(stream_idx).unwrap())
    }
}
