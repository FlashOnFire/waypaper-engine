use crate::rendering_backends::video::frames::TimedVideoFrame;
use crate::rendering_backends::video::gl::{
    ElementBuffer, GLDataType, Shader, VertexArray, VertexAttribute, VertexBuffer,
};
use crate::rendering_backends::video::pipeline::DecodingPipeline;
use crate::rendering_backends::video::utils::FrameArray;
use crate::rendering_backends::video::video_backend_consts::{
    FRAGMENT_SHADER_SRC, INDICES, VERTEX_DATA, VERTEX_SHADER_SRC,
};
use crate::wallpaper_renderer::{VideoRenderingBackend, WPRendererImpl};
use gl::types::{GLfloat, GLint, GLsizei, GLuint};
use std::ffi::c_void;
use std::path::PathBuf;
use std::ptr::null;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

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

    decoding_pipeline: DecodingPipeline,

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
        let mut decoding_pipeline = DecodingPipeline::new(&self.video_path.clone().unwrap());

        let shutdown_arc = Arc::new(AtomicBool::new(false));
        let size = decoding_pipeline.decoder_size();
        let framerate = decoding_pipeline.framerate();
        decoding_pipeline.start_decoding();

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
                decoding_pipeline,
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

        let frame = if Instant::now()
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

            // TODO: make this less bad
            let mut frames = data.decoding_pipeline.frames.lock().unwrap();

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
                // TODO: remove this unpark when we have a better way to handle frame rendering
                data.decoding_pipeline.decoding_thread.as_ref().unwrap().thread().unpark();

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
        tracing::info!("Dropping RenderData");
        unsafe {
            gl::DeleteTextures(1, &self.texture);
        }
    }
}
