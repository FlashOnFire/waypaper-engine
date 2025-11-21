use crate::profile_manager::ProfileManager;
use crate::wallpaper::Wallpaper;
use crate::wl_renderer::RenderingContext;
use crossbeam::channel::{Receiver, Sender, TryRecvError};
use linux_ipc::IpcChannel;
use std::error::Error;
use std::path::PathBuf;
use std::thread;
use waypaper_engine_shared::ipc::{IPCError, IPCRequest, IPCResponse, InternalRequest};

pub struct AppState {
    wpe_dir: PathBuf,
    rendering_context: RenderingContext,
    internal_ipc_tx: Sender<(InternalRequest, Sender<IPCResponse>)>,
    internal_ipc_rx: Receiver<(InternalRequest, Sender<IPCResponse>)>,
    profile_manager: ProfileManager,
}

impl AppState {
    pub fn new(wpe_dir: PathBuf) -> Self {
        tracing::debug!(
            "Using wallpaper engine workshop path {}",
            wpe_dir.to_string_lossy()
        );

        let (internal_ipc_tx, internal_ipc_rx) =
            crossbeam::channel::unbounded::<(InternalRequest, Sender<IPCResponse>)>();

        AppState {
            wpe_dir,
            rendering_context: RenderingContext::new(internal_ipc_tx.clone()),
            internal_ipc_tx,
            internal_ipc_rx,
            profile_manager: ProfileManager::new(),
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        ffmpeg_next::init()?;

        let internal_ipc_tx = self.internal_ipc_tx.clone();

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

                        let (req_tx, req_rx) = crossbeam::channel::unbounded::<IPCResponse>();
                        internal_ipc_tx
                            .send((InternalRequest::from(request.clone()), req_tx))
                            .unwrap();
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

        loop {
            self.rendering_context.tick();

            match self.internal_ipc_rx.try_recv() {
                Ok((req, response)) => match req {
                    InternalRequest::SetWallpaper { id, screen } => {
                        if Self::set_wallpaper(self, id, &screen, &response) {
                            self.profile_manager.save_wallpaper(id, &screen);
                        }
                    }

                    InternalRequest::ListOutputs => {
                        let outputs = self
                            .rendering_context
                            .get_outputs()
                            .drain()
                            .filter_map(|(_, output)| output.name)
                            .collect();
                        response.send(IPCResponse::Outputs(outputs))?;
                    }
                    InternalRequest::KillDaemon => {
                        unreachable!()
                    }
                    InternalRequest::NewOutput { screen } => {
                        if let Some(id) = self.profile_manager.load_wallpaper(&screen)
                            && Self::set_wallpaper(self, id, &screen, &response)
                        {
                            tracing::info!("Wallpaper [{}] loaded for screen [{}]", id, screen);
                        }
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

    fn set_wallpaper(&mut self, id: u64, screen: &str, response: &Sender<IPCResponse>) -> bool {
        let outputs = self.rendering_context.get_outputs();

        if let Some(output) = outputs.iter().find(|output| match output.1.name.as_ref() {
            Some(name) => name == screen,
            None => false,
        }) {
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
                // The wallpaper path is expected to be a directory containing wallpaper resources.
                return false;
            }

            let wallpaper = match Wallpaper::new(path.clone()) {
                Ok(wallpaper) => wallpaper,
                Err(e) => {
                    tracing::warn!("Failed to load wallpaper: {:?}", e);
                    response
                        .send(IPCResponse::Error(IPCError::WallpaperLoadingError))
                        .unwrap();
                    return false;
                }
            };
            match wallpaper {
                Wallpaper::Video { ref project, .. } => {
                    if let Some(file) = project.file.as_ref() {
                        let video_path = path.join(file);
                        if video_path.exists() {
                            tracing::info!("Found video file ! (Path : {video_path:?})");
                            self.rendering_context.set_wallpaper(output, wallpaper);
                        } else {
                            tracing::warn!("Video file does not exist: {:?}", video_path);
                            response
                                .send(IPCResponse::Error(IPCError::WallpaperNotFound))
                                .unwrap();
                            return false;
                        }
                    } else {
                        tracing::warn!("Wallpaper project file is None for video wallpaper");
                        response
                            .send(IPCResponse::Error(IPCError::WallpaperNotFound))
                            .unwrap();
                        return false;
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
                    if response
                        .send(IPCResponse::Error(IPCError::UnsupportedWallpaperType))
                        .is_err()
                    {
                        tracing::info!("Unable to send UnsupportedWallpaperType");
                    }
                    return false;
                }
            }

            tracing::info!("Set wallpaper for output [{}] with id [{}]", screen, id);

            if response.send(IPCResponse::Success).is_err() {
                tracing::info!("Unable to send Success");
            }

            true
        } else {
            tracing::warn!(
                "Received wrong output in SetWallpaper request: [{}]",
                screen
            );
            if response
                .send(IPCResponse::Error(IPCError::ScreenNotFound))
                .is_err()
            {
                tracing::warn!(
                    "Failed to send ScreenNotFound response for output [{}]: {:?}. \
                     The IPC channel may be closed; client will not receive error response.",
                    screen,
                    IPCError::ScreenNotFound
                );
            }

            false
        }
    }
}
