use crate::rendering_backends::video::deinterlacer::Deinterlacer;
use crate::rendering_backends::video::utils;
use anyhow::anyhow;
use ffmpeg_next::Rational;
use ffmpeg_next::error::EAGAIN;
use ffmpeg_next::ffi::av_frame_copy_props;
use ffmpeg_next::format::Pixel;
use ffmpeg_next::format::stream::Stream;
use ffmpeg_next::frame::Video as VideoFrame;
use ffmpeg_next::{Error, Packet, codec, software};

pub struct VideoDecoder {
    decoder: codec::decoder::Video,
    stream_time_base: Rational,
    scaler: Option<software::scaling::Context>,
    deinterlacer: Option<Deinterlacer>,
}

impl VideoDecoder {
    pub fn new(stream: &Stream) -> anyhow::Result<Self> {
        let context_decoder = codec::Context::from_parameters(stream.parameters())?;
        let mut decoder = context_decoder.decoder().video()?;

        if decoder.format() == Pixel::None || decoder.width() == 0 || decoder.height() == 0 {
            return Err(anyhow!(
                "Invalid video codec parameters: format, width, or height is not set"
            ));
        }

        let codec_params = stream.parameters();
        let time_base = stream.time_base();
        println!("Stream index: {}", stream.index());
        println!(
            "  Time base: {}/{}",
            time_base.numerator(),
            time_base.denominator()
        );
        println!("  Codec ID: {:?}", codec_params.id());
        println!("  Codec type: {:?}", codec_params.medium());
        unsafe {
            println!(
                "  Width x Height: {}x{}",
                (*codec_params.as_ptr()).width,
                (*codec_params.as_ptr()).height
            );
            println!("  Format: {:?}", (*codec_params.as_ptr()).format);
        }

        println!(
            "Decoder time base: {}/{}",
            decoder.time_base().numerator(),
            decoder.time_base().denominator()
        );

        if decoder.time_base().numerator() == 0 {
            tracing::warn!(
                "Decoder time base is zero. This will cause issues. Setting it to stream time base"
            );
            decoder.set_time_base(time_base);
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
                .map_err(|_| anyhow!("Backend error"))?,
            )
        } else {
            None
        };

        Ok(VideoDecoder {
            decoder,
            stream_time_base: stream.time_base(),
            scaler,
            deinterlacer: None,
        })
    }

    pub fn stream_time_base(&self) -> Rational {
        self.stream_time_base
    }

    pub fn decoder_time_base(&self) -> Rational {
        self.decoder.time_base()
    }

    pub fn size(&self) -> (u32, u32) {
        (self.decoder.width(), self.decoder.height())
    }

    pub fn feed(&mut self, mut packet: Packet) -> anyhow::Result<()> {
        // Rescale the packet timestamps to the decoder's time base before decoding
        packet.rescale_ts(self.stream_time_base, self.decoder.time_base());

        self.decoder.send_packet(&packet)?;
        Ok(())
    }

    pub fn receive_frames(&mut self) -> anyhow::Result<Option<Vec<VideoFrame>>> {
        let mut decoded = VideoFrame::empty();

        match self.decoder.receive_frame(&mut decoded) {
            Ok(_) => {
                tracing::debug!(
                    "Received frame: pts: {:?}, width: {}, height: {}, format: {:?}",
                    decoded.pts(),
                    decoded.width(),
                    decoded.height(),
                    decoded.format()
                );

                // if decoded.is_interlaced() {
                //     tracing::debug!("Frame is interlaced, applying deinterlacing");
                //     if self.deinterlacer.is_none() {
                //         self.deinterlacer = Some(
                //             Deinterlacer::new(
                //                 decoded.width(),
                //                 decoded.height(),
                //                 self.decoder.time_base(),
                //                 self.decoder.format().into(),
                //                 self.decoder.aspect_ratio(),
                //             )
                //             .expect("Failed to create deinterlacer"),
                //         )
                //     }
                //
                //     let deinterlacer = self.deinterlacer.as_mut().unwrap();
                //     deinterlacer.feed(&decoded)?;
                //
                //     let mut deinterlaced_frames = vec![];
                //
                //     // TODO: more robust error handling
                //     while let Ok(deinterlaced) = deinterlacer.pull() {
                //         tracing::debug!(
                //             "Deinterlaced frame: pts: {:?}, width: {}, height: {}, format: {:?}",
                //             deinterlaced.pts(),
                //             deinterlaced.width(),
                //             deinterlaced.height(),
                //             deinterlaced.format()
                //         );
                //         deinterlaced_frames.push(deinterlaced);
                //     }
                //
                //     tracing::debug!("Deinterlaced {} frames", deinterlaced_frames.len());
                //
                //     let processed_frames = self.process_decoded_frames(deinterlaced_frames).expect("Failed to process deinterlaced frames");
                //
                //     Ok(Some(processed_frames))
                // Note to self: if we want to support deinterlacing, we need to flush the deinterlacer at the end of the stream
                // } else {
                    let processed_frame = self.process_decoded_frame(decoded)?;
                    Ok(Some(vec![processed_frame]))
                // }
            }
            Err(Error::Eof) => Err(anyhow!("Read exhausted")),
            Err(Error::Other { errno }) if errno == EAGAIN => {
                tracing::info!("Decoder returned EAGAIN, waiting for more data");
                Ok(None)
            }
            _ => Err(anyhow!("Unknown error")),
        }
    }

    pub fn process_decoded_frames(
        &mut self,
        mut frames: Vec<VideoFrame>,
    ) -> anyhow::Result<Vec<VideoFrame>> {
        let mut processed_frames = Vec::with_capacity(frames.len());

        for frame in frames.drain(..) {
            let processed_frame = self.process_decoded_frame(frame)?;
            processed_frames.push(processed_frame);
        }

        Ok(processed_frames)
    }

    fn process_decoded_frame(&mut self, frame: VideoFrame) -> anyhow::Result<VideoFrame> {
        let mut scaled_frame = if let Some(ref mut scaler) = self.scaler {
            let mut scaled_frame = VideoFrame::empty();
            scaler.run(&frame, &mut scaled_frame)?;
            unsafe {
                av_frame_copy_props(scaled_frame.as_mut_ptr(), frame.as_ptr());
            }

            scaled_frame
        } else {
            frame
        };

        // Rescale the timestamps of the decoded frame to the stream time base
        if let Some(pts) = scaled_frame.pts() {
            scaled_frame.set_pts(Some(utils::rescale_q(
                pts,
                self.decoder.time_base(),
                self.stream_time_base,
            )));
        }

        // maybe rescale dts too?

        Ok(scaled_frame)
    }

    pub(crate) fn flush(&mut self) {
        tracing::info!("Flushing decoder");
        self.drain();
        self.decoder.flush();
        tracing::info!("Decoder flushed");
    }

    pub fn drain(&mut self) {
        tracing::info!("Draining decoder");
        self.decoder.send_eof().unwrap();
        let mut count = 0;
        let mut decoded = VideoFrame::empty();
        const MAX_DRAIN_ITERATIONS: u32 = 50;

        //TODO flush interlacing filter at the end of the stream if set
        loop {
            let result = self.decoder.receive_frame(&mut decoded);

            match result {
                Ok(_) => {
                    count += 1;
                    decoded = VideoFrame::empty(); // Reset the frame to ensure the previous data is dropped
                }
                Err(Error::Eof) => {
                    // Normal end of stream
                    break;
                }
                Err(e) => {
                    // Any other error - log it and stop draining
                    tracing::warn!("Error during drain: {:?}, stopping drain", e);
                    break;
                }
            }

            if count >= MAX_DRAIN_ITERATIONS {
                tracing::error!("Drain exceeded maximum iterations ({}), forcing stop", MAX_DRAIN_ITERATIONS);
                break;
            }
        }

        tracing::info!("Decoder drained ({} frames)", count);
    }
}

unsafe impl Send for VideoDecoder {}
unsafe impl Sync for VideoDecoder {}
