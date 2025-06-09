use anyhow::anyhow;
use ffmpeg_next::error::EAGAIN;
use ffmpeg_next::ffi::av_frame_copy_props;
use ffmpeg_next::format::Pixel;
use ffmpeg_next::frame::Video;
use ffmpeg_next::{codec, software, Error, Packet};
use ffmpeg_next::{Rational, Stream};
use video_rs::frame::RawFrame;

pub struct VideoDecoder {
    decoder: codec::decoder::Video,
    stream_time_base: Rational,
    scaler: Option<software::scaling::Context>,
}

impl VideoDecoder {
    pub fn new(
        codec_parameters: codec::Parameters,
        stream_time_base: Rational,
    ) -> anyhow::Result<Self> {
        let context_decoder = codec::Context::from_parameters(codec_parameters)?;
        let decoder = context_decoder.decoder().video()?;

        if decoder.format() == Pixel::None || decoder.width() == 0 || decoder.height() == 0 {
            return Err(anyhow!(
                "Invalid video codec parameters: format, width, or height is not set"
            ));
        }

        let scaler = if decoder.format() != Pixel::RGB24 {
            Some(
                software::scaling::Context::get(
                    decoder.format(),
                    decoder.width(),
                    decoder.height(),
                    Pixel::RGB24,
                    decoder.width(),
                    decoder.height(),
                    software::scaling::Flags::AREA,
                )
                .map_err(video_rs::Error::BackendError)?,
            )
        } else {
            None
        };

        Ok(VideoDecoder {
            decoder,
            stream_time_base,
            scaler,
        })
    }

    pub fn stream_time_base(&self) -> Rational {
        self.stream_time_base
    }

    pub fn decoder_time_base(&self) -> Rational {
        self.decoder.time_base()
    }

    pub fn feed(&mut self, mut packet: Packet) -> anyhow::Result<()> {
        packet.rescale_ts(self.stream_time_base, self.decoder.time_base());
        self.decoder.send_packet(&packet)?;
        Ok(())
    }

    pub fn receive_frame(&mut self, packet: Packet) -> anyhow::Result<Option<Video>> {
        self.feed(packet)?;
        tracing::info!("frame fed to decoder");
        let mut decoded = Video::empty();

        match self.decoder.receive_frame(&mut decoded) {
            Ok(_) => {
                if let Some(ref mut scaler) = self.scaler {
                    let mut frame_scaled = RawFrame::empty();
                    scaler.run(&decoded, &mut frame_scaled)?;
                    unsafe {
                        av_frame_copy_props(frame_scaled.as_mut_ptr(), decoded.as_ptr());
                    }

                    Ok(Some(frame_scaled))
                } else {
                    Ok(Some(decoded))
                }
            }
            Err(Error::Eof) => Err(anyhow!("Read exhausted")),
            Err(Error::Other { errno }) if errno == EAGAIN => {
                tracing::info!("Decoder returned EAGAIN, waiting for more data");
                Ok(None)
            }
            _ => Err(anyhow!("Unknown error")),
        }
    }

    pub fn drain(&mut self) {
        self.decoder.send_eof().unwrap();
        loop {
            let mut decoded = Video::empty();
            loop {
                if self.decoder.receive_frame(&mut decoded).is_err() {
                    break;
                }
            }
        }
    }
}
