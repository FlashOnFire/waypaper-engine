use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::mpsc::{Sender, TryRecvError};
use std::{fs, thread};

use linux_ipc::IpcChannel;
use waypaper_engine_shared::ipc::{IPCError, IPCRequest, IPCResponse};

use crate::wallpaper::Wallpaper;
use crate::wl_renderer::RenderingContext;
pub struct AppState {
    wpe_dir: PathBuf,
    rendering_context: RenderingContext,
}

const SAVE_PATH: &str = "wallpapers.conf";

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
    pub fn save_wallpaper(id: u64, screen: &str) -> std::io::Result<()> {
        let file_path = SAVE_PATH;
        let mut lines = read_save_file(file_path);

        if let Some(line) = lines.iter_mut().find(|line| line.starts_with(screen)) {
            *line = format!("{} = {}", screen, id);
        } else {
            lines.push(format!("{} = {}", screen, id));
        }

        fs::write(file_path, lines.join("\n"))?;
        tracing::info!("{}", format!("Save wallpaper : {}", lines.join("\n")));
        Ok(())
    }

    fn load_wallpaper(&mut self) -> std::io::Result<()> {
        let file_path = SAVE_PATH;
        let lines = read_save_file(file_path);

        lines
            .iter()
            .filter_map(|line| {
                line.split_once('=').and_then(|(screen_part, id_part)| {
                    id_part
                        .trim()
                        .parse::<u64>()
                        .ok()
                        .map(|id| (id, screen_part.trim().to_string()))
                })
            })
            .for_each(|(id, screen)| {
                let (tx, _rx) = mpsc::channel::<IPCResponse>();
                self.set_wallpaper(id, screen, tx);
            });

        Ok(())
    }
}

fn read_save_file(file_path: &str) -> Vec<String> {
    let content = if Path::new(file_path).exists() {
        fs::read_to_string(file_path).unwrap_or_else(|_| String::new())
    } else {
        String::new()
    };

    content.lines().map(|line| line.to_string()).collect()
}
