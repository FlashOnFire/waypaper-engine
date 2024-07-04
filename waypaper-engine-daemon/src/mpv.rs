use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use khronos_egl::{Context, Instance, Static};
use libmpv2::Mpv;
use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use smithay_client_toolkit::reexports::client::Connection;

fn get_proc_address(egl: &Rc<Instance<Static>>, name: &str) -> *mut c_void {
    egl.get_proc_address(name).unwrap() as *mut c_void
}

pub struct MpvRenderer {
    pub render_context: Option<RenderContext>,
    egl: Rc<Instance<Static>>,
    mpv: Mpv,
    connection: Rc<Connection>,
    video_path: PathBuf,
    started_playback: bool,
}

impl MpvRenderer {
    pub fn new(connection: Rc<Connection>, egl: Rc<Instance<Static>>, video_path: PathBuf) -> Self {
        let mpv = Mpv::new().expect("Error while creating mpv");

        // Setting various properties
        mpv.set_property("terminal", "yes").unwrap(); // Logs in term

        // mpv.set_property("msg-level", "all=v").unwrap(); // Verbose logs
        mpv.set_property("input-cursor", "no").unwrap(); // No cursor
        mpv.set_property("cursor-autohide", "no").unwrap(); // No cursor hiding
        mpv.set_property("config", "no").unwrap(); // Disable config loading

        // mpv.set_property("fbo-format", "rgba8").unwrap(); // FrameBuffer format (worse quality when setting it and I don't know why it works without, but it works)
        mpv.set_property("vo", "libmpv").unwrap(); // Rendering through libmpv
        mpv.set_property("hwdec", "auto").unwrap(); // Auto-Detect Hardware Decoding
        mpv.set_property("gpu-hwdec-interop", "vaapi").unwrap(); // Fix to avoid mpv crashing when libcuda is not found (todo: test if this workaround doesn't affect machines running on nvidia gpus)

        mpv.set_property("loop", "inf").unwrap(); // Play video in loop

        Self {
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
            self.render_context = Some(
                RenderContext::new(
                    self.mpv.ctx.as_mut(),
                    vec![
                        RenderParam::ApiType(RenderParamApiType::OpenGl),
                        RenderParam::InitParams(OpenGLInitParams {
                            get_proc_address,
                            ctx: self.egl.clone(),
                        }),
                        RenderParam::WaylandDisplay(
                            self.connection.backend().display_ptr() as *mut std::ffi::c_void
                        ),
                    ],
                )
                .unwrap(),
            );
        }
    }
    pub fn play_file(&self, file: &Path) {
        self.mpv
            .command(
                "loadfile",
                &[
                    &("\"".to_owned() + &file.to_string_lossy() + "\""),
                    "replace",
                ],
            )
            .unwrap();
    }

    pub fn set_speed(&self, speed: f32) {
        self.mpv
            .set_property("speed", format!("{speed:.2}"))
            .unwrap();
    }

    pub fn render(&mut self, width: u32, height: u32) {
        if !self.started_playback {
            self.play_file(&self.video_path);
            self.started_playback = true;
        }

        self.render_context
            .as_ref()
            .unwrap()
            .render::<Context>(0, i32::try_from(width).unwrap(), height as i32, true)
            .unwrap()
    }
}
