use std::ffi::{c_void, CString};
use std::path::{Path, PathBuf};
use std::ptr;
use std::ptr::null;
use std::rc::Rc;
use std::str::from_utf8;

use gl::types::{GLchar, GLenum, GLfloat, GLint, GLsizei, GLsizeiptr, GLuint};
use smithay_client_toolkit::reexports::client::Connection;
use video_rs::{Decoder, Error};

use waypaper_engine_shared::project::WallpaperType;

use crate::egl::EGLState;
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

fn compile_shader(src: &str, shader_type: GLenum) -> GLuint {
    let mut shader = 0;
    unsafe {
        shader = gl::CreateShader(shader_type);

        let c_str = CString::new(src.as_bytes()).unwrap();
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        if status != (gl::TRUE as GLint) {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            gl::GetShaderInfoLog(
                shader,
                len,
                ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            panic!("{}", from_utf8(&buf).expect("ShaderInfoLog not valid utf8"));
        }
    }
    shader
}

fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);

        let mut status = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        if status != (gl::TRUE as GLint) {
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            gl::GetProgramInfoLog(
                program,
                len,
                ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            panic!(
                "{}",
                from_utf8(&buf).expect("ProgramInfoLog not valid utf8")
            );
        }
        program
    }
}

pub struct VideoWPRenderer2 {
    connection: Rc<Connection>,
    egl_state: Rc<EGLState>,

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
    decoder: Decoder,
    texture: GLuint,
}

impl VideoWPRenderer2 {
    pub(crate) fn new(connection: Rc<Connection>, egl_state: Rc<EGLState>) -> Self {
        Self {
            connection,
            egl_state,
            render_context: None,
            video_path: None,
            started_playback: false,
        }
    }

    fn play_file(&mut self, file: &Path) {
        let source = video_rs::Location::File(self.video_path.as_ref().unwrap().clone());
        let decoder = Decoder::new(source).expect("Failed to create decoder");
        let size = decoder.size_out();

        let ctx = self.render_context.as_mut().unwrap();

        unsafe {
            if let Some(data) = &ctx.data {
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

            ctx.data = Some(RenderData { decoder, texture });
        }
    }
}

impl WPRendererImpl for VideoWPRenderer2 {
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

            gl::BindFragDataLocation(program, 0, CString::new("out_color").unwrap().as_ptr());

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
        match wp {
            Wallpaper::Video {
                ref project,
                base_dir_path,
            } => {
                self.video_path = Some(base_dir_path.join(project.file.as_ref().unwrap()));
                self.started_playback = false;
            }
            Wallpaper::Scene { .. } => {}
            Wallpaper::Web { .. } => {}
            Wallpaper::Preset { .. } => {}
        }
    }

    fn render(&mut self, width: u32, height: u32) {
        if !self.started_playback {
            self.play_file(&self.video_path.as_ref().unwrap().clone());
            self.started_playback = true;
        }

        let ctx = self.render_context.as_mut().unwrap();
        let data = ctx.data.as_mut().unwrap();

        let (time, frame) = match data.decoder.decode() {
            Ok(o) => o,
            Err(err) => match err {
                Error::ReadExhausted => {
                    data.decoder
                        .seek_to_start()
                        .expect("Error during video frames decoding");
                    data.decoder
                        .decode()
                        .expect("Error during video frames decoding")
                }
                _ => panic!("Error during video frames decoding"),
            },
        };

        unsafe {
            gl::UseProgram(ctx.program);
            gl::BindVertexArray(ctx.vao);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ctx.ebo);

            gl::BindTexture(gl::TEXTURE_2D, data.texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);

            let (width, height) = data.decoder.size_out();
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                width as GLsizei,
                height as GLsizei,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                frame.as_ptr() as *const c_void,
            );

            gl::DrawElements(
                gl::TRIANGLES,
                6,
                gl::UNSIGNED_INT,
                std::ptr::null::<c_void>(),
            );

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
            gl::UseProgram(0);
            gl::BindVertexArray(0);
        }
    }

    fn get_wp_type(&self) -> WallpaperType {
        WallpaperType::Video
    }
}

impl Drop for VideoWPRenderer2 {
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
