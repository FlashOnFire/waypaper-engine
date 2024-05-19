use std::path::{Path, PathBuf};
use std::rc::Rc;

use khronos_egl::{Context, Instance, Static};
use libmpv2::{FileState, Mpv};
use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use smithay_client_toolkit::reexports::client::Connection;

//noinspection RsUnusedImport (false positive)
use crate::egl::get_proc_address;

pub struct MpvRenderer {
    connection: Rc<Connection>,
    egl: Rc<Instance<Static>>,
    mpv: Mpv,
    video_path: PathBuf,
    pub render_context: Option<RenderContext>,
    started_playback: bool,
}

impl MpvRenderer {
    pub fn new(connection: Rc<Connection>, egl: Rc<Instance<Static>>, video_path: PathBuf) -> Self {
        let mpv = Mpv::new().expect("Error while creating mpv");

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


        MpvRenderer {
            connection,
            egl,
            mpv,
            video_path,
            render_context: None,
            started_playback: false,
        }
    }

    pub fn init_rendering_context(&mut self) {
        unsafe {
            self.render_context = Some(RenderContext::new(
                self.mpv.ctx.as_mut(),
                vec![
                    RenderParam::ApiType(RenderParamApiType::OpenGl),
                    RenderParam::InitParams(OpenGLInitParams {
                        get_proc_address,
                        ctx: self.egl.clone(),
                    }),
                    RenderParam::WaylandDisplay(self.connection.backend().display_ptr() as *mut std::ffi::c_void),
                ],
            ).unwrap());
        }
    }
    pub fn play_file(&self, file: &Path) {
        self.mpv.playlist_load_files(&[(file.to_str().unwrap(), FileState::Replace, None)]).unwrap();
    }

    pub fn set_speed(&self, speed: f32) {
        self.mpv.set_property("speed", format!("{:.2}", speed)).unwrap()
    }

    pub fn render(&mut self, width: u32, height: u32) {
        if !self.started_playback {
            self.play_file(&self.video_path);
            self.started_playback = true;
        }
        
        self.render_context.as_ref().unwrap().render::<Context>(0, width as i32, height as i32, true).unwrap()
    }
}