use ffmpeg_next::format::context::Input;
use ffmpeg_next::format::input;
use ffmpeg_next::media;
use std::path::PathBuf;

enum PacketType {
    Video,
    Audio,
}

struct Demuxer {
    ictx: Input,
    video_idx: usize,
    audio_idx: usize,
    loop_mode: bool,
}

impl Demuxer {
    fn new(path: PathBuf, loop_mode: bool) -> Result<Self, ffmpeg_next::Error> {
        let ictx = input(&path)?;

        let video_stream = ictx.streams().best(media::Type::Video).unwrap();
        let audio_stream = ictx.streams().best(media::Type::Audio).unwrap();

        let video_stream_idx = video_stream.index();
        let audio_stream_idx = audio_stream.index();

        Ok(Self {
            ictx,
            video_idx: video_stream_idx,
            audio_idx: audio_stream_idx,
            loop_mode,
        })
    }

    fn read_packet(
        &mut self,
    ) -> Result<Option<(PacketType, ffmpeg_next::Packet)>, ffmpeg_next::Error> {
        let mut error_count = 0;

        loop {
            match self.ictx.packets().next() {
                Some((stream, packet)) => {
                    if stream.index() == self.video_idx {
                        return Ok(Some((PacketType::Video, packet)));
                    } else if stream.index() == self.audio_idx {
                       return  Ok(Some((PacketType::Audio, packet)));
                    }
                }
                None => {
                    error_count += 1;
                    if error_count > 3 {
                        if self.loop_mode {
                            self.seek_to_start();
                            error_count = 0;
                        } else {
                            return Ok(None);
                        }
                    }
                }
            }
        }
    }

    fn seek_to_start(&mut self) {
        self.ictx.seek(i64::MIN, ..).unwrap();
    }
}
