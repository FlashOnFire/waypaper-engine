use crate::wallpaper::Wallpaper;
use crate::wl_renderer::RenderingContext;
use crate::WP_DIR;
use linux_ipc::IpcChannel;
use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::thread;
use waypaper_engine_shared::ipc::IPCRequest;
use waypaper_engine_shared::project::WEProject;

pub struct AppState {
    rendering_context: RenderingContext,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            rendering_context: RenderingContext::new(),
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let (tx, rx) = mpsc::channel::<IPCRequest>();

        thread::spawn(move || {
            let mut channel = IpcChannel::new("/tmp/waypaper-engine.sock").unwrap();
            tracing::info!("Started IPC channel");

            loop {
                let (response, reply) = channel
                    .receive::<IPCRequest, String>()
                    .expect("Failed to create channel");

                tracing::debug!("Received msg : [{:?}]", response);
                tx.send(response).unwrap();
            }
        });

        loop {
            match rx.try_recv() {
                Ok(req) => match req {
                    IPCRequest::SetWP { id, screen } => {
                        let outputs = self.rendering_context.get_outputs();
                        if let Some(output) = outputs
                            .iter()
                            .find(|output| output.1.name.as_ref().unwrap() == &screen)
                        {
                            let path = Path::new(WP_DIR).join(id.to_string());
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
                                    let path = Path::new(WP_DIR)
                                        .join(project.workshop_id.unwrap().to_string())
                                        .join(project.file.as_ref().unwrap());

                                    if path.exists() {
                                        tracing::info!("Found video file ! (Path : {path:?})");

                                        self.rendering_context.set_wallpaper(output, wallpaper);
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
        }
    }
}

fn add_id(proj: &mut WEProject, filename: &OsStr) -> Result<(), Box<dyn Error>> {
    if proj.workshop_id.is_none() {
        proj.workshop_id = Some(u64::from_str(filename.to_str().unwrap())?);
    }

    Ok(())
}
