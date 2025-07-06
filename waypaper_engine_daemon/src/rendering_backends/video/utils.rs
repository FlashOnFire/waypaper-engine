use anyhow::anyhow;
use ffmpeg_next::ffi::{av_image_copy_to_buffer, AVPixelFormat};
use ffmpeg_next::filter::Graph;
use ffmpeg_next::frame::Video as VideoFrame;
use ffmpeg_next::Rational;
use ndarray::Array3;
pub type FrameArray = Array3<u8>;

// Shamelessly stolen from the `video_rs` crate

/// Converts an RGB24 video `AVFrame` produced by ffmpeg to an `ndarray`.
///
/// # Arguments
///
/// * `frame` - Video frame to convert.
///
/// # Return value
///
/// A three-dimensional `ndarray` with dimensions `(H, W, C)` and type byte.
pub fn convert_frame_to_ndarray_rgb24(frame: &mut VideoFrame) -> anyhow::Result<FrameArray> {
    unsafe {
        let frame_ptr = frame.as_mut_ptr();
        let frame_width: i32 = (*frame_ptr).width;
        let frame_height: i32 = (*frame_ptr).height;
        let frame_format =
            std::mem::transmute::<std::ffi::c_int, AVPixelFormat>((*frame_ptr).format);
        assert_eq!(frame_format, AVPixelFormat::AV_PIX_FMT_RGB24);

        let mut frame_array =
            FrameArray::default((frame_height as usize, frame_width as usize, 3_usize));

        let bytes_copied = av_image_copy_to_buffer(
            frame_array.as_mut_ptr(),
            frame_array.len() as i32,
            (*frame_ptr).data.as_ptr() as *const *const u8,
            (*frame_ptr).linesize.as_ptr(),
            frame_format,
            frame_width,
            frame_height,
            1,
        );

        if bytes_copied == frame_array.len() as i32 {
            Ok(frame_array)
        } else {
            Err(anyhow!(
                "Error occurred while copying frame data to ndarray"
            ))
        }
    }
}

pub fn make_yadif_filter_graph(
    width: u32,
    height: u32,
    time_base: Rational,
    pix_fmt: AVPixelFormat,
    aspect_ratio: Rational,
) -> Result<Graph, ffmpeg_next::Error> {
    let mut graph = Graph::new();
    let args = &format!(
        "video_size={}x{}:pix_fmt={}:time_base={}/{}:pixel_aspect={}/{}",
        width,
        height,
        pix_fmt as i32,
        time_base.numerator(),
        time_base.denominator(),
        aspect_ratio.numerator(),
        aspect_ratio.denominator()
    );
    dbg!(args);
    graph.add(&ffmpeg_next::filter::find("buffer").unwrap(), "in", args)?;

    let args = "mode=1:parity=auto:deint=interlaced";
    graph.add(&ffmpeg_next::filter::find("yadif").unwrap(), "yadif", args)?;
    graph.add(&ffmpeg_next::filter::find("buffersink").unwrap(), "out", "")?;

    graph
        .get("in")
        .unwrap()
        .link(0, &mut graph.get("yadif").unwrap(), 0);
    graph
        .get("yadif")
        .unwrap()
        .link(0, &mut graph.get("out").unwrap(), 0);
    graph.validate()?;

    Ok(graph)
}

/// Equivalent to av_rescale_q from FFmpeg
pub(crate) fn rescale_q(value: i64, src: Rational, dst: Rational) -> i64 {
    value * dst.numerator() as i64 * src.denominator() as i64
        / (dst.denominator() as i64 * src.numerator() as i64)
}


