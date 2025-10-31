use std::ffi::c_void;
use std::rc::Rc;

use khronos_egl::{self as egl, Dynamic};
use khronos_egl::{Config, Context, Display, Instance, Surface};
use smithay_client_toolkit::reexports::client::Connection;

pub fn get_proc_address(
    egl: &Rc<Instance<Dynamic<libloading::Library, egl::EGL1_5>>>,
    name: &str,
) -> *mut c_void {
    egl.get_proc_address(name).unwrap() as *mut c_void
}

pub struct EGLState {
    _wl_connection: Rc<Connection>,
    pub(crate) egl: Rc<Instance<Dynamic<libloading::Library, egl::EGL1_5>>>,
    pub(crate) egl_display: Display,
    pub(crate) egl_context: Context,
    pub(crate) config: Config,
}

impl EGLState {
    pub fn new(connection: Rc<Connection>) -> Self {
        let lib =
            unsafe { libloading::Library::new("libEGL.so.1").expect("unable to find libEGL.so.1") };
        let egl = unsafe {
            egl::DynamicInstance::<egl::EGL1_5>::load_required_from(lib)
                .expect("unable to load libEGL.so.1")
        };

        let egl = Rc::new(egl);

        unsafe {
            let egl_display = egl
                .get_display(connection.backend().display_ptr() as *mut c_void)
                .expect("Failed to get EGL Display");
            egl.initialize(egl_display)
                .expect("Couldn't initialize EGL Display");

            #[rustfmt::skip]
            let attributes = [
                egl::SURFACE_TYPE, egl::WINDOW_BIT,
                egl::RED_SIZE, 8,
                egl::GREEN_SIZE, 8,
                egl::BLUE_SIZE, 8,
                egl::RENDERABLE_TYPE, egl::OPENGL_BIT,
                egl::NONE,
            ];

            egl.bind_api(egl::OPENGL_API).unwrap();

            #[rustfmt::skip]
            let context_attributes = [
                egl::CONTEXT_MAJOR_VERSION, 4,
                egl::CONTEXT_MINOR_VERSION, 6,
                egl::CONTEXT_OPENGL_PROFILE_MASK, egl::CONTEXT_OPENGL_CORE_PROFILE_BIT,
                egl::NONE,
            ];

            let config = egl
                .choose_first_config(egl_display, &attributes)
                .expect("unable to find an appropriate ELG configuration")
                .expect("No EGL configuration found");

            let egl_context = egl
                .create_context(egl_display, config, None, &context_attributes)
                .expect("Could not create context");

            gl::load_with(|str| get_proc_address(&egl, str));

            Self {
                _wl_connection: connection,
                egl,
                egl_display,
                egl_context,
                config,
            }
        }
    }

    pub fn attach_context(&self, surface: Surface) {
        self.egl
            .make_current(
                self.egl_display,
                Some(surface),
                Some(surface),
                Some(self.egl_context),
            )
            .unwrap();
    }

    pub fn detach_context(&self) {
        self.egl
            .make_current(self.egl_display, None, None, None)
            .unwrap();
    }
}

impl Drop for EGLState {
    fn drop(&mut self) {
        self.egl
            .destroy_context(self.egl_display, self.egl_context)
            .expect("Couldn't destroy EGL Context");

        self.egl
            .terminate(self.egl_display)
            .expect("Couldn't destroy EGL Display");
    }
}
