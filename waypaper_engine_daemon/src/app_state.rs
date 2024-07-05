use std::error::Error;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::thread;

use linux_ipc::IpcChannel;

use waypaper_engine_shared::ipc::IPCRequest;
use waypaper_engine_shared::project::WEProject;

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
        let (tx, rx) = mpsc::channel::<IPCRequest>();
        let (stop_tx, stop_rx) = oneshot::channel::<()>();

        let ipc_thread = thread::spawn(move || {
            let mut channel = IpcChannel::new("/tmp/waypaper-engine.sock").unwrap();
            tracing::info!("Started IPC channel");

            loop {
                match channel.receive::<IPCRequest, String>() {
                    Ok((response, reply)) => {
                        tracing::debug!("Received msg : [{:?}]", response);
                        tx.send(response).unwrap();
                    }
                    Err(err) => tracing::warn!("IPC Received invalid data (Error: {})", err),
                }

                if stop_rx.try_recv().is_ok() {
                    break;
                }
            }
        });

        let mut i = 0;
        loop {
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
                                let mut wallpaper = Wallpaper::new(
                                    self.rendering_context.connection.clone(),
                                    &self.rendering_context.egl_state,
                                    &path,
                                )
                                .unwrap();
                                let filename = path.file_name().unwrap();

                                match wallpaper {
                                    Wallpaper::Video {
                                        ref mut project, ..
                                    } => add_id(project, filename)?,
                                    Wallpaper::Scene {
                                        ref mut project, ..
                                    } => add_id(project, filename)?,
                                    Wallpaper::Web { ref mut project } => {
                                        add_id(project, filename)?
                                    }
                                    Wallpaper::Preset { ref mut project } => {
                                        add_id(project, filename)?
                                    }
                                }

                                if let Wallpaper::Video { ref project, .. } = wallpaper {
                                    let path = self
                                        .wpe_dir
                                        .join(project.workshop_id.unwrap().to_string())
                                        .join(project.file.as_ref().unwrap());

                                    if path.exists() {
                                        tracing::info!("Found video file ! (Path : {path:?})");

                                        self.rendering_context.set_wallpaper(output, wallpaper);
                                        i += 1;
                                    }
                                }
                            }
                        }
                    }
                },
                Err(err) => match err {
                    TryRecvError::Empty => {}
                    TryRecvError::Disconnected => panic!(),
                },
            }

            self.rendering_context.tick();
            
            if i > 8 {
                break;
            }
        }

        stop_tx.send(()).unwrap();
        ipc_thread.join().unwrap();

        Ok(())
    }
}

fn add_id(proj: &mut WEProject, filename: &OsStr) -> Result<(), Box<dyn Error>> {
    if proj.workshop_id.is_none() {
        proj.workshop_id = Some(u64::from_str(filename.to_str().unwrap())?);
    }

    Ok(())
}
