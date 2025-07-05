use anyhow::anyhow;
use ffmpeg_next::ffi::{av_image_copy_to_buffer, AVPixelFormat};
use ndarray::Array3;
use ffmpeg_next::frame::Video as VideoFrame;
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
            Err(anyhow!("Error occurred while copying frame data to ndarray"))
        }
    }
}