use gstreamer::ffi::{gst_bin_add, gst_bin_add_many, gst_context_new, gst_context_writable_structure, gst_element_factory_make, gst_element_link, gst_element_link_many, gst_element_set_context, gst_element_set_state, gst_init, gst_is_initialized, gst_pipeline_new, GST_STATE_PLAYING, gst_structure_set, GstBin, GstElement};
use gstreamer::glib::ffi::{GFALSE, gpointer, GTRUE};
use gstreamer_gl::ffi::{gst_context_set_gl_display, GST_GL_API_OPENGL, gst_gl_context_new, gst_gl_context_new_wrapped, GST_GL_CONTEXT_TYPE_EGL, GST_GL_DISPLAY_CONTEXT_TYPE, gst_gl_display_new_with_type, GST_GL_DISPLAY_TYPE_WAYLAND, GST_GL_PLATFORM_EGL, GstGLDisplay};
use gstreamer_gl::gst_video::ffi::{gst_video_overlay_set_render_rectangle, GstVideoOverlay};
use smithay_client_toolkit::reexports::client::Connection;
use std::ffi::{c_char, c_int, c_void, CString};
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::rc::Rc;
use gstreamer::glib::gobject_ffi::{g_object_set, g_object_set_property, GObject, GValue};
use gstreamer_gl_egl::ffi::gst_gl_display_egl_new_with_egl_display;
use gstreamer_gl_wayland::ffi::gst_gl_display_wayland_new_with_display;
use waypaper_engine_shared::project::WallpaperType;

use crate::egl::EGLState;
use crate::wallpaper::Wallpaper;
use crate::wallpaper_renderer::WPRendererImpl;

pub struct GStreamerVideoWPRenderer {
    _connection: Rc<Connection>,
    _egl_state: Rc<EGLState>,

    render_context: Option<RenderContext>,

    video_path: Option<PathBuf>,
    started_playback: bool,
}

struct RenderContext {
    pipeline: *mut GstElement,
    filesrc: *mut GstElement,
    decodebin3: *mut GstElement,

}

impl GStreamerVideoWPRenderer {
    pub(crate) fn new(connection: Rc<Connection>, egl_state: Rc<EGLState>) -> Self {
        Self {
            _connection: connection,
            _egl_state: egl_state,
            render_context: None,
            video_path: None,
            started_playback: false,
        }
    }

    fn start_playback(&mut self) {}
}
impl WPRendererImpl for GStreamerVideoWPRenderer {
    fn init_render(&mut self) {
        unsafe {
            if gst_is_initialized() == GFALSE {
                gst_init(
                    null_mut::<c_int>(),
                    null_mut::<*mut *mut c_char>(),
                );
                tracing::info!("GStreamer init");
            } else {
                tracing::info!("GStreamer was already init");
            }

            let pipeline_cstr = CString::new("pipeline").unwrap();
            let pipeline = gst_pipeline_new(pipeline_cstr.as_ptr());

            let filesrc_cstr = CString::new("filesrc").unwrap();
            let src = gst_element_factory_make(filesrc_cstr.as_ptr(), filesrc_cstr.as_ptr());
            let location_cstr = CString::new("location").unwrap();
            let src_cstr = CString::new("/home/flashonfire/.steam/steam/steamapps/workshop/content/431960/3228410350/Video_240420222304_Segment(1).mp4").unwrap();
            g_object_set(
                src as *mut GObject,
                location_cstr.as_ptr(),
                src_cstr.as_ptr(),
                null_mut::<c_char>()
            );

            let db3_cstr = CString::new("decodebin3").unwrap();
            let decodebin3 = gst_element_factory_make(db3_cstr.as_ptr(), db3_cstr.as_ptr());


            let videoconvert_cstr = CString::new("videoconvert").unwrap();
            let videoconvert = gst_element_factory_make(videoconvert_cstr.as_ptr(), videoconvert_cstr.as_ptr());
            
            //let gst_gl_display = gst_gl_display_wayland_new_with_display(self._connection.backend().display_ptr() as *mut c_void);
            let gst_gl_display = gst_gl_display_egl_new_with_egl_display(self._egl_state.egl_display.as_ptr());

            let gst_gl_ctx = gst_gl_context_new_wrapped(gst_gl_display as *mut GstGLDisplay,
                                                        self._egl_state.egl_context.as_ptr() as usize,
                                                        GST_GL_PLATFORM_EGL,
                                                        GST_GL_API_OPENGL
            );
            let gl_is_cstr = CString::new("glimagesink").unwrap();
            let gl_imagesink = gst_element_factory_make(gl_is_cstr.as_ptr(), gl_is_cstr.as_ptr());
            gst_video_overlay_set_render_rectangle(
                gl_imagesink as *mut GstVideoOverlay,
                0 as c_int,
                0 as c_int,
                3840 as c_int,
                2160 as c_int,
            );

            /*let ctx_cstr = CString::new("other-context").unwrap();
            g_object_set(gl_imagesink as *mut GObject, ctx_cstr.as_ptr(), gst_gl_ctx as *const GValue);*/
            let context = gst_context_new(GST_GL_CONTEXT_TYPE_EGL as *const [u8] as *const c_char, GTRUE);
            let s = gst_context_writable_structure(context);
            let ctx_cstr = CString::new("context").unwrap();
            gst_structure_set(s, ctx_cstr.as_ptr(), GST_GL_CONTEXT_TYPE_EGL, gst_gl_ctx, null::<c_char>());
            
            gst_element_set_context(gl_imagesink, context);


            if decodebin3.is_null() {//|| gst_gl_display.is_null() || gst_gl_ctx.is_null() || gl_imagesink.is_null() {
                tracing::error!("Could not create video decoding pipeline");
            }

            gst_bin_add_many(pipeline as *mut GstBin, src, decodebin3, videoconvert, gl_imagesink, null_mut::<GstElement>());

            if gst_element_link_many(src, decodebin3, null_mut::<c_char>()) == GFALSE {
                tracing::warn!("Failed to link elements!");
            }
            if gst_element_link_many(videoconvert, gl_imagesink, null_mut::<c_char>()) == GFALSE {
                tracing::warn!("Failed to link elements!");
            }

            gst_element_set_state(pipeline, GST_STATE_PLAYING);



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
            _ => unreachable!(),
        }
    }

    fn render(&mut self, width: u32, height: u32) {
        if !self.started_playback {
            self.start_playback();
            self.started_playback = true;
        }
    }

    fn get_wp_type(&self) -> WallpaperType {
        WallpaperType::Video
    }
}

impl Drop for GStreamerVideoWPRenderer {
    fn drop(&mut self) {}
}
