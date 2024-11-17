use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use khronos_egl::{Context, Instance, Static};
use libmpv2::Mpv;
use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use smithay_client_toolkit::reexports::client::Connection;

use waypaper_engine_shared::project::WallpaperType;

use crate::egl::EGLState;
use crate::wallpaper::Wallpaper;
use crate::wallpaper_renderer::WPRendererImpl;

fn get_proc_address(egl: &Rc<Instance<Static>>, name: &str) -> *mut c_void {
    egl.get_proc_address(name).unwrap() as *mut c_void
}

pub struct MPVVideoWPRenderer {
    connection: Rc<Connection>,
    egl_state: Rc<EGLState>,
    
    render_context: Option<RenderContext>,
    mpv: Option<Mpv>,
    
    video_path: Option<PathBuf>,
    started_playback: bool,
}

impl MPVVideoWPRenderer {
    pub(crate) fn new(connection: Rc<Connection>, egl_state: Rc<EGLState>) -> Self {
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
            egl_state,
            render_context: None,
            mpv: Some(mpv),
            video_path: None,
            started_playback: false,
        }
    }
    
    fn play_file(&self, file: &Path) {
        self.mpv.as_ref().unwrap()
            .command(
                "loadfile",
                &[
                    &("\"".to_owned() + &file.to_string_lossy() + "\""),
                    "replace",
                ],
            )
            .unwrap();
    }
}

impl WPRendererImpl for MPVVideoWPRenderer {
    fn init_render(&mut self) {
        unsafe {
            self.render_context = Some(
                RenderContext::new(
                    self.mpv.as_mut().unwrap().ctx.as_mut(),
                    vec![
                        RenderParam::ApiType(RenderParamApiType::OpenGl),
                        RenderParam::InitParams(OpenGLInitParams {
                            get_proc_address,
                            ctx: self.egl_state.egl.clone(),
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

    fn setup_wallpaper(&mut self, wp: &Wallpaper) {
        match wp {
            Wallpaper::Video { ref project, base_dir_path } => {
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
            self.play_file(self.video_path.as_ref().unwrap());
            self.started_playback = true;
        }

        self.render_context
            .as_ref()
            .unwrap()
            .render::<Context>(0, i32::try_from(width).unwrap(), height as i32, true)
            .unwrap();
    }

    fn get_wp_type(&self) -> WallpaperType {
        WallpaperType::Video
    }
}
