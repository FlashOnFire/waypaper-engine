use gl::types::{GLchar, GLenum, GLint};
use std::ffi::{CString, c_void};
use std::ptr;
use std::str::from_utf8;

pub struct VertexArray {
    id: u32,
    ebo: ElementBuffer,
    vbos: Vec<VertexBuffer>,
}

impl VertexArray {
    pub fn new(ebo: ElementBuffer) -> Self {
        let mut id = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut id);
        }
        let vao = Self {
            id,
            ebo,
            vbos: Vec::new(),
        };

        vao.bind();
        vao.ebo.bind();
        vao.unbind();

        vao
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindVertexArray(self.id);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::BindVertexArray(0);
        }
    }

    pub fn bind_vertex_buffer(&mut self, vbo: VertexBuffer) {
        vbo.bind();
        vbo.bind_vertex_attributes();
        self.vbos.push(vbo)
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        unsafe {
            tracing::debug!("Deleting vertex array with id: {}", self.id);
            gl::DeleteVertexArrays(1, &self.id);
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum GLDataType {
    Float = gl::FLOAT as isize,
    Int = gl::INT as isize,
    UnsignedInt = gl::UNSIGNED_INT as isize,
}

pub struct VertexAttribute {
    pub(crate) index: u32,
    pub(crate) size: i32,
    pub(crate) data_type: GLDataType,
    pub(crate) normalized: bool,
    pub(crate) stride: GLint,
    pub(crate) offset: usize,
}

pub struct VertexBuffer {
    id: u32,
    vertex_attributes: Vec<VertexAttribute>,
}

impl VertexBuffer {
    pub fn new<T>(data: &[T]) -> Self {
        let mut id = 0;
        unsafe {
            gl::GenBuffers(1, &mut id);
        }
        let vbo = Self {
            id,
            vertex_attributes: Vec::new(),
        };

        vbo.bind();
        vbo.buffer_data(data);
        vbo.unbind();

        vbo
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.id);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }
    }

    pub fn buffer_data<T>(&self, data: &[T]) {
        unsafe {
            gl::BufferData(
                gl::ARRAY_BUFFER,
                size_of_val(data) as isize,
                data.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
        }
    }

    pub fn buffer_sub_data<T>(&self, offset: isize, data: &[T]) {
        unsafe {
            gl::BufferSubData(
                gl::ARRAY_BUFFER,
                offset,
                size_of_val(data) as isize,
                data.as_ptr() as *const _,
            );
        }
    }

    pub fn add_vertex_attribute(&mut self, attribute: VertexAttribute) {
        self.vertex_attributes.push(attribute);
    }

    pub fn bind_vertex_attributes(&self) {
        for attr in &self.vertex_attributes {
            unsafe {
                gl::VertexAttribPointer(
                    attr.index,
                    attr.size,
                    attr.data_type as GLenum,
                    if attr.normalized { gl::TRUE } else { gl::FALSE },
                    attr.stride,
                    attr.offset as *const c_void,
                );
                gl::EnableVertexAttribArray(attr.index);
            }
        }
    }
}

impl Drop for VertexBuffer {
    fn drop(&mut self) {
        unsafe {
            tracing::debug!("Deleting vertex buffer with id: {}", self.id);
            gl::DeleteBuffers(1, &self.id);
        }
    }
}

pub struct ElementBuffer {
    id: u32,
}

impl ElementBuffer {
    pub fn new<T>(indices: &[T]) -> Self {
        let mut id = 0;
        unsafe {
            gl::GenBuffers(1, &mut id);
        }
        let ebo = Self { id };

        ebo.bind();
        unsafe {
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                size_of_val(indices) as isize,
                indices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
        }
        ebo.unbind();

        ebo
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.id);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
        }
    }
}

impl Drop for ElementBuffer {
    fn drop(&mut self) {
        unsafe {
            tracing::debug!("Deleting element buffer with id: {}", self.id);
            gl::DeleteBuffers(1, &self.id);
        }
    }
}

pub struct Shader {
    id: u32,
}

impl Shader {
    pub fn new(vertex_shader_source: &str, fragment_shader_source: &str) -> Self {
        let vertex_shader = Shader::compile_shader(gl::VERTEX_SHADER, vertex_shader_source);
        let fragment_shader = Shader::compile_shader(gl::FRAGMENT_SHADER, fragment_shader_source);

        let id = unsafe {
            let program = gl::CreateProgram();
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::LinkProgram(program);

            gl::ValidateProgram(program);
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

            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);

            program
        };

        Self { id }
    }

    fn compile_shader(shader_type: u32, source: &str) -> u32 {
        let shader = unsafe { gl::CreateShader(shader_type) };
        let c_str = CString::new(source).unwrap();
        unsafe {
            gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
            gl::CompileShader(shader);
        }

        let mut status = gl::FALSE as GLint;
        unsafe {
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
        }
        if status != (gl::TRUE as GLint) {
            let mut len = 0;
            unsafe {
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            }
            let mut buf = Vec::with_capacity(len as usize);
            unsafe {
                gl::GetShaderInfoLog(
                    shader,
                    len,
                    ptr::null_mut(),
                    buf.as_mut_ptr() as *mut GLchar,
                );
            }
            panic!("{}", from_utf8(&buf).expect("ShaderInfoLog not valid utf8"));
        }

        shader
    }

    pub fn use_program(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::UseProgram(0);
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            tracing::debug!("Deleting shader program with id: {}", self.id);
            gl::DeleteProgram(self.id);
        }
    }
}
