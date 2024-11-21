use gl::types::{GLfloat, GLint};

pub(crate) const THREAD_FRAME_BUFFER_SIZE: usize = 20;

#[rustfmt::skip]
pub(crate) const VERTEX_DATA: [GLfloat; 32] = [
     1.0,  1.0,  0.0,    1.0, 0.0, 0.0,    1.0, 1.0,
     1.0, -1.0,  0.0,    0.0, 1.0, 0.0,    1.0, 0.0,
    -1.0, -1.0,  0.0,    0.0, 0.0, 1.0,    0.0, 0.0,
    -1.0,  1.0,  0.0,    0.0, 0.0, 1.0,    0.0, 1.0,
];

#[rustfmt::skip]
pub(crate) const INDICES: [GLint; 6] = [
    0, 1, 3,
    1, 2, 3,
];

pub(crate) const VERTEX_SHADER_SRC: &str = r#"
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

pub(crate) const FRAGMENT_SHADER_SRC: &str = r#"
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
