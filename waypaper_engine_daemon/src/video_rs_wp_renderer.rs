use std::cell::OnceCell;
use std::collections::VecDeque;
use std::ffi::{c_void, CString};
use std::path::PathBuf;
use std::ptr::null;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;

use gl::types::{GLfloat, GLint, GLsizei, GLsizeiptr, GLuint};
use smithay_client_toolkit::reexports::client::Connection;
use video_rs::hwaccel::HardwareAccelerationDeviceType;
use video_rs::{Decoder, DecoderBuilder, Error, Frame, Time};

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
    texture: GLuint,

    framerate: f32,
    size: (u32, u32),

    last_frame_time: Instant,
    last_frame: Frame,

    decoding_thread_handle: OnceCell<JoinHandle<()>>,
    frames: Arc<Mutex<VecDeque<Frame>>>,

    shutdown: Arc<AtomicBool>,
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

        let shutdown_arc = Arc::new(AtomicBool::new(false));

        let (thread_handle, frames_vec) = start_decoding_thread(decoder, shutdown_arc.clone());

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
                last_frame: frames_vec.lock().unwrap().front().unwrap().clone(),
                decoding_thread_handle: OnceCell::from(thread_handle),
                frames: frames_vec.clone(),
                shutdown: shutdown_arc,
            });
        }
    }
}

fn start_decoding_thread(
    mut decoder: Decoder,
    shutdown: Arc<AtomicBool>,
) -> (JoinHandle<()>, Arc<Mutex<VecDeque<Frame>>>) {
    let mut frames_vec = VecDeque::with_capacity(20);
    frames_vec.push_back(decoder.decode().unwrap().1);
    let frames_arc = Arc::new(Mutex::new(frames_vec)); // init with first frame

    let weak = Arc::downgrade(&frames_arc);

    let (mut decoder_split, mut reader, stream_index) = decoder.into_parts();

    let handle = thread::spawn(move || {
        tracing::debug!("Spawn decoding thread");
        'outer: while !shutdown.load(Ordering::Relaxed) {
            let packet_result = reader.read(stream_index);
            
            let packet = match packet_result {
                Ok(packet) => packet,
                Err(err) => match err {
                    Error::ReadExhausted => {
                        tracing::debug!("Video ended, seeking to start");
                        reader.seek_to_start().unwrap();
                        continue;
                    },
                    _ => panic!("Error while decoding video frame"),
                },
            };
            
            let (time, frame) = match decoder_split.decode(packet).expect("Failed to decode video frame") {
                Some(v) => v,
                None => continue,
            };

            while !shutdown.load(Ordering::Relaxed) {
                if let Some(strong) = weak.upgrade() {
                    if strong.lock().unwrap().len() >= 20 {
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
                frames_vec.push_back(frame);
            } else {
                break 'outer;
            }
        }
        
        tracing::debug!("Got out of the decoding loop, draining decoder");
        while let Ok(option) = decoder_split.drain_raw() && option.is_some() {
            tracing::debug!("Draining frame");
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

        let frame = if Instant::now()
            .duration_since(data.last_frame_time)
            .as_secs_f32()
            < 1.0 / data.framerate
        {
            tracing::debug!("Not enough time since last frame, rendering last frame again");
            &data.last_frame
        } else {
            data.last_frame_time = Instant::now();

            let mut frames = data.frames.lock().unwrap();
            match frames.pop_front() {
                Some(frame) => {
                    data.decoding_thread_handle.get().unwrap().thread().unpark();
                    data.last_frame = frame;
                    &data.last_frame
                }
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

impl Drop for RenderContext {
    fn drop(&mut self) {
        unsafe {
            tracing::debug!("Destroying video render context");
            gl::DeleteBuffers(1, &self.ebo);
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteProgram(self.program);
        }
    }
}

impl Drop for RenderData {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.frames.lock().unwrap().clear();
        self.decoding_thread_handle.get().unwrap().thread().unpark();

        let _ = self.decoding_thread_handle.take().unwrap().join();
        unsafe {
            gl::DeleteTextures(1, &self.texture);
        }
    }
}
