use crate::rendering_backends::video::decoder::VideoDecoder;
use crate::rendering_backends::video::demuxer::{Demuxer, TimedPacket};
use crate::rendering_backends::video::frames::{OrderedFramesContainer, TimedVideoFrame};
use crate::rendering_backends::video::gl::{
    ElementBuffer, GLDataType, Shader, VertexArray, VertexAttribute, VertexBuffer,
};
use crate::rendering_backends::video::video_backend_consts::{
    FRAGMENT_SHADER_SRC, INDICES, THREAD_FRAME_BUFFER_SIZE, VERTEX_DATA, VERTEX_SHADER_SRC,
};
use crate::wallpaper_renderer::{VideoRenderingBackend, WPRendererImpl};
use gl::types::{GLfloat, GLint, GLsizei, GLuint};
use std::cell::OnceCell;
use std::ffi::c_void;
use std::path::PathBuf;
use std::ptr::null;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;

use crate::rendering_backends::video::utils;
use crate::rendering_backends::video::utils::FrameArray;

pub struct VideoWPRenderer {
    render_context: Option<RenderContext>,

    video_path: Option<PathBuf>,
    started_playback: bool,
}

struct RenderContext {
    shader: Shader,
    vao: VertexArray,
    data: Option<RenderData>,
}

struct RenderData {
    texture: GLuint,

    framerate: f32,
    size: (u32, u32),

    last_frame_time: Instant,
    last_frame: Option<FrameArray>,

    decoding_thread_handle: OnceCell<JoinHandle<()>>,
    frames: Arc<Mutex<OrderedFramesContainer<TimedVideoFrame>>>, // Using BinaryHeap to keep frames in order of their timestamps

    shutdown: Arc<AtomicBool>,
}

impl VideoWPRenderer {
    pub(crate) fn new() -> Self {
        Self {
            render_context: None,
            video_path: None,
            started_playback: false,
        }
    }

    fn start_playback(&mut self) {
        let mut demuxer = Demuxer::new(self.video_path.clone().unwrap().as_path())
            .expect("Failed to create demuxer");
        let video_stream = demuxer
            .video_stream()
            .expect("No video stream found in demuxer");

        let framerate = (video_stream.avg_frame_rate().numerator() as f32)
            / (video_stream.avg_frame_rate().denominator() as f32);

        let decoder =
            VideoDecoder::new(video_stream.parameters(), video_stream.time_base()).unwrap();
        let size = decoder.size();

        let shutdown_arc = Arc::new(AtomicBool::new(false));
        let (thread_handle, frames_vec) =
            start_decoding_thread(demuxer, decoder, shutdown_arc.clone());

        let ctx = self.render_context.as_mut().unwrap();

        unsafe {
            let mut texture: GLuint = 0;
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);

            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGB as GLint,
                size.0 as GLsizei,
                size.1 as GLsizei,
                0,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                null(),
            );

            ctx.data.replace(RenderData {
                texture,
                framerate,
                size,
                last_frame_time: Instant::now(),
                last_frame: None,
                decoding_thread_handle: OnceCell::from(thread_handle),
                frames: frames_vec,
                shutdown: shutdown_arc,
            });
        }
    }
}

impl VideoRenderingBackend for VideoWPRenderer {
    fn setup_video_wallpaper(&mut self, video_path: PathBuf) {
        tracing::info!("Setup video_rs wp");

        self.video_path = Some(video_path);
        self.started_playback = false;
    }
}

fn start_decoding_thread(
    mut demuxer: Demuxer,
    mut decoder: VideoDecoder,
    shutdown: Arc<AtomicBool>,
) -> (
    JoinHandle<()>,
    Arc<Mutex<OrderedFramesContainer<TimedVideoFrame>>>,
) {
    let frames_vec: OrderedFramesContainer<TimedVideoFrame> =
        OrderedFramesContainer::with_capacity(THREAD_FRAME_BUFFER_SIZE);

    let frames_arc = Arc::new(Mutex::new(frames_vec));

    let weak = Arc::downgrade(&frames_arc);

    let handle = thread::spawn(move || {
        let mut rewind_count: u32 = 0;

        'outer: while !shutdown.load(Ordering::Relaxed) {
            let (packet, _time) = loop {
                let packet_result: Option<TimedPacket> = demuxer.read();

                match packet_result {
                    Some(timed_packed) => match timed_packed {
                        TimedPacket::Video(packet, time) => break (packet, time),
                        _ => continue, // Skip non-video packets
                    },
                    None => {
                        tracing::debug!("Video ended, seeking to start");
                        demuxer.seek_to_start().unwrap();
                        rewind_count += 1;
                        continue;
                    }
                };
            };

            tracing::info!(
                "Decoding packet with PTS: {}, time base: {:?}",
                packet.pts().unwrap(),
                packet.time_base()
            );

            let timed_frame = match decoder.receive_frame(packet) {
                Ok(Some(mut frame)) => {
                    let timestamp = frame.timestamp();

                    TimedVideoFrame::new(
                        utils::convert_frame_to_ndarray_rgb24(&mut frame).unwrap(),
                        timestamp,
                        rewind_count,
                    )
                }
                Ok(None) => {
                    tracing::debug!("No frame received, waiting for more data");
                    continue;
                }
                Err(e) => {
                    tracing::error!("Failed to decode video frame: {}", e);
                    continue;
                }
            };

            while !shutdown.load(Ordering::Relaxed) {
                if let Some(strong) = weak.upgrade() {
                    if strong.lock().unwrap().len() >= THREAD_FRAME_BUFFER_SIZE {
                        tracing::debug!("Frames in queue >= 20, paused decoding");
                        thread::park();
                        tracing::debug!("Resumed decoding")
                    } else {
                        break;
                    };
                } else {
                    break 'outer;
                }
            }

            if let Some(strong) = weak.upgrade() {
                let mut frames_vec = strong.lock().unwrap();
                frames_vec.push(timed_frame);
            } else {
                break 'outer;
            }
        }

        tracing::debug!("Got out of the decoding loop, draining decoder");
        decoder.drain();

        tracing::debug!("Exited decoding Thread!");
    });

    (handle, frames_arc)
}

impl WPRendererImpl for VideoWPRenderer {
    fn init_render(&mut self) {
        let ebo = ElementBuffer::new(&INDICES);
        let mut vao = VertexArray::new(ebo);
        let mut vbo = VertexBuffer::new(&VERTEX_DATA);

        vbo.add_vertex_attribute(VertexAttribute {
            index: 0,
            size: 3,
            data_type: GLDataType::Float,
            normalized: false,
            stride: (5 * size_of::<GLfloat>()) as GLint,
            offset: 0,
        });

        vbo.add_vertex_attribute(VertexAttribute {
            index: 1,
            size: 2,
            data_type: GLDataType::Float,
            normalized: false,
            stride: (5 * size_of::<GLfloat>()) as GLint,
            offset: 3 * size_of::<GLfloat>(),
        });

        vao.bind();
        vao.bind_vertex_buffer(vbo);
        vao.unbind();

        let shader = Shader::new(VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC);

        self.render_context = Some(RenderContext {
            shader,
            vao,
            data: None,
        })
    }

    fn render(&mut self, width: u32, height: u32) {
        if !self.started_playback {
            self.start_playback();
            self.started_playback = true;
        }

        let ctx = self.render_context.as_mut().unwrap();
        let data = ctx.data.as_mut().unwrap();

        let frame= if Instant::now()
            .duration_since(data.last_frame_time)
            .as_secs_f32()
            < 1.0 / data.framerate
        {
            tracing::debug!("Not enough time since last frame, rendering last frame again");
            match data.last_frame.as_ref() {
                Some(frame) => frame,
                None => {
                    tracing::warn!("No last frame available, cannot render");
                    return;
                }
            }
        } else {
            data.last_frame_time = Instant::now();

            let mut frames = data.frames.lock().unwrap();

            if frames.is_empty() || frames.len() < 16 {
                tracing::debug!("Not enough frames in queue, rendering last frame");
                match data.last_frame.as_ref() {
                    Some(frame) => frame,
                    None => {
                        tracing::warn!("No last frame available, cannot render");
                        return;
                    }
                }
            } else {
                let TimedVideoFrame {
                    rewind_count,
                    timestamp,
                    frame,
                } = frames.pop().unwrap();
                data.decoding_thread_handle.get().unwrap().thread().unpark();

                tracing::info!(
                    "Rendering new frame, frames in queue: {}, rewind count: {}, frame time: {:?}",
                    frames.len(),
                    rewind_count,
                    timestamp
                );
                data.last_frame = Some(frame);
                data.last_frame.as_ref().unwrap()
            }
        };

        unsafe {
            // Reset viewport each frame to avoid problems when rendering on two screens with different resolutions
            gl::Viewport(0, 0, width as GLsizei, height as GLsizei);

            ctx.shader.use_program();
            ctx.vao.bind();

            gl::BindTexture(gl::TEXTURE_2D, data.texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);

            let (tex_width, tex_height) = data.size;
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                tex_width as GLsizei,
                tex_height as GLsizei,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                frame.as_ptr() as *const c_void,
            );

            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, null::<c_void>());

            ctx.vao.unbind();
            ctx.shader.unbind();
        }
    }
}

impl Drop for RenderData {
    fn drop(&mut self) {
        tracing::info!("Dropping RenderData, shutting down decoding thread");
        self.shutdown.store(true, Ordering::Relaxed);
        self.frames.lock().unwrap().clear();
        self.decoding_thread_handle.get().unwrap().thread().unpark();

        let _ = self.decoding_thread_handle.take().unwrap().join();
        unsafe {
            gl::DeleteTextures(1, &self.texture);
        }
    }
}
