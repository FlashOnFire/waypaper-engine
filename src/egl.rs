use std::ffi::c_void;
use std::rc::Rc;
use khronos_egl::{Config, Context, Display, Instance, Static};
use smithay_client_toolkit::reexports::client::Connection;

use khronos_egl as egl;

pub fn get_proc_address(egl: &Rc<Instance<Static>>, name: &str) -> *mut c_void {
    egl.get_proc_address(name).unwrap() as *mut c_void
}

pub struct EGLState {
    pub(crate) egl: Rc<Instance<Static>>,
    pub(crate) egl_display: Display,
    pub(crate) egl_context: Context,
    pub(crate) config: Config,
}

impl EGLState {
    pub fn new(connection: &Connection) -> Self {
        let egl = Rc::new(Instance::new(Static));

        unsafe {
            let egl_display = egl.get_display(connection.backend().display_ptr() as *mut c_void).unwrap();
            egl.initialize(egl_display).unwrap();

            let attributes = [
                egl::RED_SIZE, 8,
                egl::GREEN_SIZE, 8,
                egl::BLUE_SIZE, 8,
                egl::RENDERABLE_TYPE, egl::OPENGL_BIT,
                egl::NONE
            ];

            egl.bind_api(egl::OPENGL_API).unwrap();

            let context_attributes = [
                egl::CONTEXT_MAJOR_VERSION, 4,
                egl::CONTEXT_MINOR_VERSION, 6,
                egl::CONTEXT_OPENGL_PROFILE_MASK, egl::CONTEXT_OPENGL_CORE_PROFILE_BIT,
                egl::NONE
            ];

            let config = egl
                .choose_first_config(egl_display, &attributes)
                .expect("unable to find an appropriate ELG configuration")
                .expect("No EGL configuration found");

            let egl_context = egl
                .create_context(egl_display, config, None, &context_attributes)
                .expect("Could not create context");

            gl::load_with(|str| get_proc_address(&egl, str));
            
            EGLState {
                egl,
                egl_display,
                egl_context,
                config,
            }
        }
    }
}