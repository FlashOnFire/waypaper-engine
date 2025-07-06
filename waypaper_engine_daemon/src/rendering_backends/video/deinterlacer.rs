use crate::rendering_backends::video::utils;
use ffmpeg_next::ffi::AVPixelFormat;
use ffmpeg_next::filter::graph::Graph;
use ffmpeg_next::frame::Video as VideoFrame;
use ffmpeg_next::{Error, Rational};

pub struct Deinterlacer {
    graph: Graph,
}

impl Deinterlacer {
    pub(crate) fn feed(&mut self, frame: &VideoFrame) -> Result<(), Error> {
        self.graph
            .get("in")
            .expect("Deinterlacer input node not found")
            .source()
            .add(&frame)
    }

    pub(crate) fn pull(&mut self) -> Result<VideoFrame, Error> {
        let mut frame = VideoFrame::empty();

        self.graph
            .get("out")
            .expect("Deinterlacer output node not found")
            .sink()
            .frame(&mut frame)
            .map(|_| frame)
    }
}

impl Deinterlacer {
    pub fn new(
        width: u32,
        height: u32,
        decoder_time_base: Rational,
        pix_fmt: AVPixelFormat,
        sample_aspect_ratio: Rational,
    ) -> Result<Self, Error> {
        let graph = utils::make_yadif_filter_graph(
            width,
            height,
            decoder_time_base,
            pix_fmt,
            sample_aspect_ratio,
        )?;

        Ok(Self { graph })
    }
}
