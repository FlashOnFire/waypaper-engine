use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, TryRecvError};
use std::{env, fs, thread};

use linux_ipc::IpcChannel;
use std::collections::HashMap;
use std::fs::File;
use waypaper_engine_shared::ipc::{IPCError, IPCRequest, IPCResponse};

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

        let (new_output_tx, new_output_rx) = crossbeam::channel::unbounded();
        thread::spawn(move || {
            loop {
                println!("new output: {:?}", new_output_rx.recv().expect("oui"));
            }
        });

        AppState {
            wpe_dir,
            rendering_context: RenderingContext::new(new_output_tx),
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        ffmpeg_next::init()?;

        let (tx, rx) = mpsc::channel::<(IPCRequest, Sender<IPCResponse>)>();

        let ipc_thread = thread::spawn(move || {
            let mut channel = IpcChannel::new("/tmp/waypaper-engine.sock").unwrap();
            tracing::info!("Started IPC channel");

            loop {
                match channel.receive::<IPCRequest, IPCResponse>() {
                    Ok((request, reply)) => {
                        tracing::debug!("Received msg : [{:?}]", request);

                        if let IPCRequest::KillDaemon = request {
                            break;
                        }

                        let (req_tx, req_rx) = mpsc::channel::<IPCResponse>();
                        tx.send((request.clone(), req_tx)).unwrap();
                        match req_rx.recv() {
                            Ok(response) => {
                                tracing::debug!("Sending response : [{:?}]", response);
                                if let Err(err) = reply(response) {
                                    tracing::warn!("Failed to send IPC response: {}", err);
                                }
                            }
                            Err(err) => {
                                tracing::warn!("Failed to compute IPC response: {}", err);
                                if let Err(err) = reply(IPCResponse::Error(IPCError::InternalError))
                                {
                                    tracing::warn!("Failed to send IPC error response: {}", err);
                                }
                            }
                        }
                    }
                    Err(err) => tracing::warn!("IPC Received invalid data (Error: {})", err),
                }
            }
        });

        self.load_wallpaper()
            .expect("Unable to load wallpapers configuration");

        loop {
            self.rendering_context.tick();

            match rx.try_recv() {
                Ok((req, response)) => match req {
                    IPCRequest::SetWallpaper { id, screen } => {
                        if Self::set_wallpaper(self, id, screen.clone(), response) {
                            Self::save_wallpaper(id, &screen).expect("Unable to save wallpaper");
                        }
                    }
                    IPCRequest::ListOutputs => {
                        let outputs = self
                            .rendering_context
                            .get_outputs()
                            .drain()
                            .filter_map(|(_, output)| output.name)
                            .collect();
                        response.send(IPCResponse::Outputs(outputs)).unwrap();
                    }
                    IPCRequest::KillDaemon => {
                        unreachable!()
                    }
                },
                Err(err) => match err {
                    TryRecvError::Empty => {}
                    TryRecvError::Disconnected => break, // Daemon stopped
                },
            }
        }

        ipc_thread.join().unwrap();

        tracing::info!("Daemon stopped");

        Ok(())
    }

    fn set_wallpaper(&mut self, id: u64, screen: String, response: Sender<IPCResponse>) -> bool {
        let outputs = self.rendering_context.get_outputs();

        if let Some(output) = outputs
            .iter()
            .find(|output| output.1.name.as_ref().unwrap() == &screen)
        {
            let path = self.wpe_dir.join(id.to_string());

            if !path.exists() {
                tracing::warn!("Wallpaper path does not exist: {:?}", path);
                response
                    .send(IPCResponse::Error(IPCError::WallpaperNotFound))
                    .unwrap();
                return false;
            }

            if !path.is_dir() {
                tracing::warn!("Wallpaper path is not a directory: {:?}", path);
                response
                    .send(IPCResponse::Error(IPCError::WallpaperNotFound))
                    .unwrap();
                return false; // not sure about this
            }

            let wallpaper = Wallpaper::new(path).expect("no path found");
            let path = self.wpe_dir.join(id.to_string());
            match wallpaper {
                Wallpaper::Video { ref project, .. } => {
                    let video_path = path.join(project.file.as_ref().unwrap());

                    if video_path.exists() {
                        tracing::info!("Found video file ! (Path : {video_path:?})");

                        self.rendering_context.set_wallpaper(output, wallpaper);
                    }
                }
                Wallpaper::Scene { .. } => {
                    let scene_pkg_file = path.join("scene.pkg");

                    if scene_pkg_file.exists() {
                        tracing::info!("Found scene package file ! (Path : {scene_pkg_file:?})");

                        self.rendering_context.set_wallpaper(output, wallpaper);
                    }
                }
                _ => {
                    tracing::warn!(
                        "Unsupported wallpaper type for SetWallpaper request: [{}]",
                        screen
                    );
                    response
                        .send(IPCResponse::Error(IPCError::UnsupportedWallpaperType))
                        .unwrap();
                    return false;
                }
            }

            tracing::info!("Set wallpaper for output [{}] with id [{}]", screen, id);
            response.send(IPCResponse::Success).unwrap();

            true
        } else {
            tracing::warn!(
                "Received wrong output in SetWallpaper request: [{}]",
                screen
            );
            response
                .send(IPCResponse::Error(IPCError::ScreenNotFound))
                .unwrap();

            false
        }
    }
    pub fn save_wallpaper(id: u64, screen: &str) -> Result<(), Box<dyn Error>> {
        match wallpapers_config() {
            Ok(mut lines) => {
                lines.insert(screen.to_string(), id);
                let save_path = save_path()?;
                let file = File::create(&save_path)?;
                serde_json::to_writer_pretty(file, &lines)?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn load_wallpaper(&mut self) -> Result<(), Box<dyn Error>> {
        match wallpapers_config() {
            Ok(lines) => {
                let (tx, _rx) = mpsc::channel::<IPCResponse>();
                lines.iter().for_each(|(screen, &id)| {
                    self.set_wallpaper(id, screen.to_owned(), tx.clone());
                });
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

fn wallpapers_config() -> Result<HashMap<String, u64>, Box<dyn Error>> {
    match save_path() {
        Ok(file_path) => match File::open(&file_path) {
            Ok(file) => Ok(serde_json::from_reader(file).unwrap_or_default()),
            Err(e) => Err(Box::new(e)),
        },
        Err(e) => Err(e),
    }
}

fn save_path() -> Result<String, Box<dyn Error>> {
    let base_dir = if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_config)
    } else if let Ok(home) = env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Impossible de déterminer le répertoire de configuration",
        )));
    };

    let parent = base_dir.join("waypaper_engine");
    if !parent.exists() {
        fs::create_dir_all(&parent)?;
    }

    let file_path = parent.join("wallpapers.conf");
    Ok(file_path.to_string_lossy().into_owned())
}
