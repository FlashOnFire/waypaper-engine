use std::collections::VecDeque;
use std::ffi::{c_void, CString};
use std::path::PathBuf;
use std::ptr::null;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use gl::types::{GLfloat, GLint, GLsizei, GLsizeiptr, GLuint};
use smithay_client_toolkit::reexports::client::Connection;
use video_rs::hwaccel::HardwareAccelerationDeviceType;
use video_rs::{Decoder, DecoderBuilder, Error, Frame};

use waypaper_engine_shared::project::WallpaperType;

use crate::egl::EGLState;
use crate::gl_utils::{compile_shader, link_program};
use crate::wallpaper::Wallpaper;
use crate::wallpaper_renderer::WPRendererImpl;

#[rustfmt::skip]
static VERTEX_DATA: [GLfloat; 32] = [
     1.0,  1.0,  0.0,    1.0, 0.0, 0.0,    1.0, 1.0,
     1.0, -1.0,  0.0,    0.0, 1.0, 0.0,    1.0, 0.0,
    -1.0, -1.0,  0.0,    0.0, 0.0, 1.0,    0.0, 0.0,
    -1.0,  1.0,  0.0,    0.0, 0.0, 1.0,    0.0, 1.0,
];

#[rustfmt::skip]
static INDICES: [GLint; 6] = [
    0, 1, 3,
    1, 2, 3,
];

const VERTEX_SHADER_SRC: &str = r#"
    #version 330 core

    layout (location = 0) in vec3 aPos;
    layout (location = 1) in vec3 aColor;
    layout (location = 2) in vec2 aTexCoord;

    out vec3 color;
    out vec2 tex_coord;

    void main()
    {
        gl_Position = vec4(aPos.x, -aPos.y, aPos.z, 1.0);
        color = aColor;
        tex_coord = aTexCoord;
    }
"#;

const FRAGMENT_SHADER_SRC: &str = r#"
    #version 330 core

    out vec4 out_color;

    in vec3 color;
    in vec2 tex_coord;

    uniform sampler2D tex;

    void main()
    {
        out_color = texture(tex, tex_coord);
    }
"#;

pub struct VideoRSWPRenderer {
    _connection: Rc<Connection>,
    _egl_state: Rc<EGLState>,

    render_context: Option<RenderContext>,

    video_path: Option<PathBuf>,
    started_playback: bool,
}

struct RenderContext {
    vbo: GLuint,
    program: GLuint,
    vao: GLuint,
    ebo: GLuint,
    data: Option<RenderData>,
}

struct RenderData {
    decoding_thread_handle: JoinHandle<()>,
    frames: Arc<Mutex<VecDeque<Frame>>>,
    texture: GLuint,
    size: (u32, u32),
    framerate: f32,
    last_frame_time: Instant,
    last_frame: Frame,
}

impl VideoRSWPRenderer {
    pub(crate) fn new(connection: Rc<Connection>, egl_state: Rc<EGLState>) -> Self {
        Self {
            _connection: connection,
            _egl_state: egl_state,
            render_context: None,
            video_path: None,
            started_playback: false,
        }
    }

    fn start_playback(&mut self) {
        let source = video_rs::Location::File(self.video_path.as_ref().unwrap().clone());
        //let decoder = Decoder::new(source).expect("Failed to create decoder");

        let decoder = DecoderBuilder::new(source)
            .with_hardware_acceleration(HardwareAccelerationDeviceType::VaApi)
            .build()
            .expect("Failed to create decoder");
        let size = decoder.size_out();
        let framerate = decoder.frame_rate();

        let (thread_handle, frames_vec) = start_decoding_thread(decoder);

        let ctx = self.render_context.as_mut().unwrap();

        unsafe {
            if let Some(data) = ctx.data.take() {
                gl::DeleteTextures(1, &data.texture);
            }

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

            ctx.data = Some(RenderData {
                decoding_thread_handle: thread_handle,
                frames: frames_vec.clone(),
                texture,
                size,
                framerate,
                last_frame_time: Instant::now(),
                last_frame: frames_vec.lock().unwrap().front().unwrap().clone(),
            });
        }
    }
}

fn start_decoding_thread(mut decoder: Decoder) -> (JoinHandle<()>, Arc<Mutex<VecDeque<Frame>>>) {
    let mut frames_vec = VecDeque::with_capacity(5);
    frames_vec.push_back(decoder.decode().unwrap().1);
    let frames_arc = Arc::new(Mutex::new(frames_vec)); // init with first frame

    let weak = Arc::downgrade(&frames_arc);
    
    let handle = thread::spawn(move || {
        tracing::debug!("Spawn decoding thread");
        'outer: loop {
            let frame = match decoder.decode() {
                Ok(o) => o.1,
                Err(err) => match err {
                    Error::ReadExhausted => {
                        decoder.seek_to_start().unwrap();
                        decoder.decode().unwrap().1
                    }
                    _ => panic!("Error while decoding video frames"),
                },
            };

            while let Some(strong) = weak.upgrade()
                && strong.lock().unwrap().len() >= 20
            {
                tracing::debug!("Frames in queue >= 20, paused decoding");
                thread::park();
                tracing::debug!("Resumed decoding")
            }

            if let Some(strong) = weak.upgrade() {
                let mut frames_vec = strong.lock().unwrap();
                assert!(frames_vec.len() < 20);
                frames_vec.push_back(frame);
            } else {
                break 'outer;
            }
        }
        tracing::debug!("Exited decoding Thread!");
    });

    (handle, frames_arc)
}

impl WPRendererImpl for VideoRSWPRenderer {
    fn init_render(&mut self) {
        unsafe {
            let mut vao: GLuint = 0;
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            let mut vbo: GLuint = 0;
            gl::GenBuffers(1, &mut vbo);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (VERTEX_DATA.len() * std::mem::size_of::<GLfloat>()) as GLsizeiptr,
                std::mem::transmute(&VERTEX_DATA[0]),
                gl::STATIC_DRAW,
            );

            let vertex_shader: GLuint = compile_shader(VERTEX_SHADER_SRC, gl::VERTEX_SHADER);
            let fragment_shader = compile_shader(FRAGMENT_SHADER_SRC, gl::FRAGMENT_SHADER);

            let program = link_program(vertex_shader, fragment_shader);

            let pointer = CString::new("out_color").unwrap();
            gl::BindFragDataLocation(program, 0, pointer.as_ptr());

            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);

            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                (8 * std::mem::size_of::<GLfloat>()) as GLsizei,
                null(),
            );
            gl::EnableVertexAttribArray(0);

            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                (8 * std::mem::size_of::<GLfloat>()) as GLsizei,
                (3 * std::mem::size_of::<GLfloat>()) as *const c_void,
            );
            gl::EnableVertexAttribArray(1);

            gl::VertexAttribPointer(
                2,
                2,
                gl::FLOAT,
                gl::FALSE,
                (8 * std::mem::size_of::<GLfloat>()) as GLsizei,
                (6 * std::mem::size_of::<GLfloat>()) as *const c_void,
            );
            gl::EnableVertexAttribArray(2);

            let mut ebo: GLuint = 0;
            gl::GenBuffers(1, &mut ebo);

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (INDICES.len() * std::mem::size_of::<GLfloat>()) as GLsizeiptr,
                std::mem::transmute(&INDICES[0]),
                gl::STATIC_DRAW,
            );

            self.render_context = Some(RenderContext {
                program,
                vao,
                vbo,
                ebo,
                data: None,
            })
        }
    }

    fn setup_wallpaper(&mut self, wp: &Wallpaper) {
        tracing::debug!("Setup video_rs wp");

        match wp {
            Wallpaper::Video {
                ref project,
                base_dir_path,
            } => {
                self.video_path = Some(base_dir_path.join(project.file.as_ref().unwrap()));
                self.started_playback = false;
            }
            _ => unreachable!(),
        }
    }

    fn render(&mut self, width: u32, height: u32) {
        if !self.started_playback {
            self.start_playback();
            self.started_playback = true;
        }

        let ctx = self.render_context.as_mut().unwrap();
        let data = ctx.data.as_mut().unwrap();
        
        let frame = if Instant::now().duration_since(data.last_frame_time).as_secs_f32() < 1.0 / data.framerate {
            tracing::debug!("Not enough time since last frame, rendering last frame again");
            &data.last_frame
        } else {
            data.last_frame_time = Instant::now();

            let mut frames = data.frames.lock().unwrap();
            match frames.pop_front() {
                Some(frame) => {
                    data.decoding_thread_handle.thread().unpark();
                    data.last_frame = frame;
                    &data.last_frame
                },
                None => {
                    tracing::debug!("No frame to render in queue! Rendering last frame");
                    &data.last_frame
                }
            }
        };

        unsafe {
            // Reset viewport each frame to avoid problems when rendering on two screens with different resolutions
            gl::Viewport(0, 0, width as GLsizei, height as GLsizei);

            gl::BindVertexArray(ctx.vao);
            gl::UseProgram(ctx.program);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ctx.ebo);

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

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
            gl::UseProgram(0);
            gl::BindVertexArray(0);
        }
    }

    fn get_wp_type(&self) -> WallpaperType {
        WallpaperType::Video
    }
}

impl Drop for VideoRSWPRenderer {
    fn drop(&mut self) {
        if let Some(ctx) = self.render_context.as_mut() {
            unsafe {
                if let Some(data) = &ctx.data {
                    gl::DeleteTextures(1, &data.texture);
                }

                gl::DeleteBuffers(1, &ctx.ebo);
                gl::DeleteBuffers(1, &ctx.vbo);
                gl::DeleteVertexArrays(1, &ctx.vao);
                gl::DeleteProgram(ctx.program);
            }
        }
    }
}
