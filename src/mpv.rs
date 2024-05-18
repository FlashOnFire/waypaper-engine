use std::path::PathBuf;
use std::rc::Rc;
use khronos_egl::{Instance, Static};
use libmpv2::{FileState, Mpv};
use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use smithay_client_toolkit::reexports::client::Connection;

//noinspection RsUnusedImport (false positive)
use crate::egl::get_proc_address;

pub struct MpvRenderer {
    mpv: Mpv,
    pub render_context: RenderContext,
}

impl MpvRenderer {
    pub fn new(connection: Rc<Connection>, egl: Rc<Instance<Static>>) -> Self {
        let mut mpv = Mpv::new().expect("Error while creating mpv");

        // Setting various properties
        mpv.set_property("terminal", "yes").unwrap(); // Logs in term
        //mpv.set_property("msg-level", "all=v").unwrap(); // Verbose logs
        mpv.set_property("input-cursor", "no").unwrap(); // No cursor
        mpv.set_property("cursor-autohide", "no").unwrap(); // No cursor hiding
        mpv.set_property("config", "no").unwrap(); // Disable config loading
        //mpv.set_property("fbo-format", "rgba8").unwrap(); // FrameBuffer format (worse quality when setting it and i don't know why it works without but it works)
        mpv.set_property("vo", "libmpv").unwrap(); // Rendering through libmpv
        mpv.set_property("hwdec", "auto").unwrap(); // Auto-Detect Hardware Decoding

        mpv.set_property("loop", "inf").unwrap(); // Play video in loop

        unsafe {
            let render_context = RenderContext::new(
                mpv.ctx.as_mut(),
                vec![
                    RenderParam::ApiType(RenderParamApiType::OpenGl),
                    RenderParam::InitParams(OpenGLInitParams {
                        get_proc_address,
                        ctx: egl,
                    }),
                    RenderParam::WaylandDisplay(connection.backend().display_ptr() as *mut std::ffi::c_void),
                ],
            ).unwrap();

            MpvRenderer {
                mpv,
                render_context,
            }
        }
    }

    pub fn play_file(&self, file: PathBuf) {
        self.mpv.playlist_load_files(&[(file.to_str().unwrap(), FileState::Replace, None)]).unwrap();
    }

    pub fn set_speed(&self, speed: f32) {
        self.mpv.set_property("speed", format!("{:.2}", speed)).unwrap()
    }
}