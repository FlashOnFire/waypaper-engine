use std::error::Error;
use std::ffi::{c_char, c_int, CString};
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::thread;

use gstreamer::ffi::{
    gst_bin_add, gst_bin_add_many, gst_element_factory_make, gst_element_link,
    gst_element_link_many, gst_element_set_state, gst_init, gst_is_initialized, gst_pipeline_new,
    GstBin, GstElement, GST_STATE_PLAYING,
};
use gstreamer::glib::ffi::GFALSE;
use gstreamer::glib::gobject_ffi::{g_object_set, GObject, GValue};
use gstreamer_gl::ffi::{
    gst_gl_context_new, gst_gl_display_new_with_type, GST_GL_DISPLAY_TYPE_WAYLAND,
};
use gstreamer_gl::gst_video::ffi::{gst_video_overlay_set_render_rectangle, GstVideoOverlay};
use linux_ipc::IpcChannel;

use waypaper_engine_shared::ipc::IPCRequest;

use crate::wallpaper::Wallpaper;
use crate::wl_renderer::RenderingContext;

pub struct AppState {
    wpe_dir: PathBuf,
    rendering_context: RenderingContext,
}

impl AppState {
    pub fn new(wpe_dir: PathBuf) -> Self {
        tracing::debug!(
            "Using wallpaper engine workshop path {}",
            wpe_dir.to_string_lossy()
        );

        AppState {
            wpe_dir,
            rendering_context: RenderingContext::new(),
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        video_rs::init().unwrap();

        let (tx, rx) = mpsc::channel::<IPCRequest>();

        let ipc_thread = thread::spawn(move || {
            let mut channel = IpcChannel::new("/tmp/waypaper-engine.sock").unwrap();
            tracing::info!("Started IPC channel");

            loop {
                match channel.receive::<IPCRequest, String>() {
                    Ok((response, reply)) => {
                        tracing::debug!("Received msg : [{:?}]", response);
                        tx.send(response.clone()).unwrap();
                        if let IPCRequest::StopDaemon = response {
                            break;
                        }
                    }
                    Err(err) => tracing::warn!("IPC Received invalid data (Error: {})", err),
                }
            }
        });

        loop {
            self.rendering_context.tick();

            match rx.try_recv() {
                Ok(req) => match req {
                    IPCRequest::SetWP { id, screen } => {
                        let outputs = self.rendering_context.get_outputs();
                        if let Some(output) = outputs
                            .iter()
                            .find(|output| output.1.name.as_ref().unwrap() == &screen)
                        {
                            let path = self.wpe_dir.join(id.to_string());
                            if path.exists() && path.is_dir() {
                                let wallpaper = Wallpaper::new(path).unwrap();

                                if let Wallpaper::Video { ref project, .. } = wallpaper {
                                    let path = self
                                        .wpe_dir
                                        .join(project.workshop_id.unwrap().to_string())
                                        .join(project.file.as_ref().unwrap());

                                    if path.exists() {
                                        tracing::info!("Found video file ! (Path : {path:?})");

                                        self.rendering_context.set_wallpaper(output, wallpaper);
                                    }
                                }
                            }
                        } else {
                            tracing::warn!(
                                "Received wrong output in SetWallpaper request: [{}]",
                                screen
                            );
                        }
                    }
                    IPCRequest::StopDaemon => {
                        break;
                    }
                },
                Err(err) => match err {
                    TryRecvError::Empty => {}
                    TryRecvError::Disconnected => panic!(),
                },
            }
        }

        ipc_thread.join().unwrap();

        Ok(())
    }
}
