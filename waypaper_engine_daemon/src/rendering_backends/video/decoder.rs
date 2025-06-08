use anyhow::anyhow;
use ffmpeg_next::error::EAGAIN;
use ffmpeg_next::frame::Video;
use ffmpeg_next::{codec, Error, Packet};
use ffmpeg_next::{Rational, Stream};

struct VideoDecoder {
    decoder: codec::decoder::Video,
    stream_time_base: Rational,
}

impl VideoDecoder {
    pub fn new(input: Stream) -> anyhow::Result<Self> {
        let context_decoder = codec::Context::from_parameters(input.parameters())?;
        let decoder = context_decoder.decoder().video()?;
        let stream_time_base = input.time_base();
        Ok(VideoDecoder {
            decoder,
            stream_time_base,
        })
    }

    pub fn receive_frame(&mut self, packet: &Packet) -> anyhow::Result<Video> {
        self.decoder.send_packet(packet)?;
        let mut decoded = Video::empty();
        loop {
            match self.decoder.receive_frame(&mut decoded) {
                Ok(_) => return Ok(decoded),
                Err(Error::Eof) => return Err(anyhow!("Read exhausted")),
                Err(Error::Other { errno }) if errno == EAGAIN => continue,
                _ => return Err(anyhow!("Unknown error")),
            }
        }
    }

    pub fn drain(&mut self) {
        self.decoder.send_eof().unwrap();
        loop {
            let mut decoded = Video::empty();
            loop {
                if (self.decoder.receive_frame(&mut decoded).is_err()) {
                    break;
                }
            }
        }
    }
}
