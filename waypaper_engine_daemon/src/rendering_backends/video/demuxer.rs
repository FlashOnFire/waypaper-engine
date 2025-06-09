use ffmpeg_next::format::context::Input;
use ffmpeg_next::format::input;
use ffmpeg_next::media;
use std::path::{Path};
use video_rs::Time;

pub enum TimedPacket {
    Video(ffmpeg_next::Packet, Time),
    Audio(ffmpeg_next::Packet, Time),
}

pub struct Demuxer {
    ictx: Input,
    video_stream_idx: Option<usize>,
    audio_stream_idx: Option<usize>,
    loop_mode: bool,
}

impl Demuxer {
    pub fn new(path: &Path, loop_mode: bool) -> Result<Self, ffmpeg_next::Error> {
        let ictx = input(path)?;

        let video_stream = ictx.streams().best(media::Type::Video);
        let audio_stream = ictx.streams().best(media::Type::Audio);

        let video_stream_idx = video_stream.map(|s| s.index());
        let audio_stream_idx = audio_stream.map(|s| s.index());
        
        if video_stream_idx.is_none() && audio_stream_idx.is_none() {
            return Err(ffmpeg_next::Error::StreamNotFound);
        }

        Ok(Self {
            ictx,
            video_stream_idx,
            audio_stream_idx,
            loop_mode,
        })
    }

    pub fn read(
        &mut self,
    ) -> Option<TimedPacket> {
        let mut error_count = 0;

        loop {
            match self.ictx.packets().next() {
                Some((stream, packet)) => {
                    // Let chains will be stabilized in rust 1.88
                    // if let Some(video_idx) = self.video_stream_idx && stream.index() == video_idx {
                    //     return Ok(Some((PacketType::Video, packet)));
                    // } else if let Some(audio_idx) = self.audio_stream_idx && stream.index() == audio_idx {
                    //     return Ok(Some((PacketType::Audio, packet)));
                    // }
                    let time = packet.pts();
                    if self.video_stream_idx.is_some_and(|video_idx| stream.index() == video_idx) {
                        return Some(TimedPacket::Video(packet, Time::new(time, stream.time_base())));
                    } else if self.audio_stream_idx.is_some_and(|audio_idx| stream.index() == audio_idx) {
                        return Some(TimedPacket::Audio(packet, Time::new(time, stream.time_base())));
                    }
                }
                None => {
                    error_count += 1;
                    if error_count > 3 {
                        if self.loop_mode {
                            self.seek_to_start().unwrap();
                            error_count = 0;
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
    }

    pub fn seek_to_start(&mut self) -> Result<(), ffmpeg_next::Error> {
        self.ictx.seek(i64::MIN, ..)
    }
}
