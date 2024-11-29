use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::thread;

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
        video_rs::init()?;

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
                                let path = self
                                    .wpe_dir
                                    .join(id.to_string());
                                match wallpaper {
                                    Wallpaper::Video { ref project, .. } => {
                                        let video_path = path.join(project.file.as_ref().unwrap());

                                        if video_path.exists() {
                                            tracing::info!("Found video file ! (Path : {video_path:?})");

                                            self.rendering_context.set_wallpaper(output, wallpaper);
                                        }
                                    },
                                    Wallpaper::Scene {..} => {
                                        let scene_pkg_file = path.join("scene.pkg");

                                        if scene_pkg_file.exists() {
                                            tracing::info!("Found scene package file ! (Path : {scene_pkg_file:?})");

                                            self.rendering_context.set_wallpaper(output, wallpaper);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        } else {
                            tracing::warn!("Received wrong output in SetWallpaper request: [{}]", screen);
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
