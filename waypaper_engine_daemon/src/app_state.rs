use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, TryRecvError};
use std::thread;

use linux_ipc::IpcChannel;
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

        loop {
            self.rendering_context.tick();

            match rx.try_recv() {
                Ok((req, response)) => match req {
                    IPCRequest::SetWallpaper { id, screen } => {
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
                                continue;
                            }

                            if !path.is_dir() {
                                tracing::warn!("Wallpaper path is not a directory: {:?}", path);
                                response
                                    .send(IPCResponse::Error(IPCError::WallpaperNotFound))
                                    .unwrap();
                                continue;
                            }

                            let wallpaper = Wallpaper::new(path)?;
                            let path = self.wpe_dir.join(id.to_string());
                            match wallpaper {
                                Wallpaper::Video { ref project, .. } => {
                                    let video_path = path.join(project.file.as_ref().unwrap());

                                    if video_path.exists() {
                                        tracing::info!(
                                            "Found video file ! (Path : {video_path:?})"
                                        );

                                        self.rendering_context.set_wallpaper(output, wallpaper);
                                    }
                                }
                                Wallpaper::Scene { .. } => {
                                    let scene_pkg_file = path.join("scene.pkg");

                                    if scene_pkg_file.exists() {
                                        tracing::info!(
                                            "Found scene package file ! (Path : {scene_pkg_file:?})"
                                        );

                                        self.rendering_context.set_wallpaper(output, wallpaper);
                                    }
                                }
                                _ => {
                                    tracing::warn!(
                                        "Unsupported wallpaper type for SetWallpaper request: [{}]",
                                        screen
                                    );
                                    response
                                        .send(IPCResponse::Error(
                                            IPCError::UnsupportedWallpaperType,
                                        ))
                                        .unwrap();
                                    continue;
                                }
                            }

                            tracing::info!(
                                "Set wallpaper for output [{}] with id [{}]",
                                screen,
                                id
                            );
                            response.send(IPCResponse::Success).unwrap();
                        } else {
                            tracing::warn!(
                                "Received wrong output in SetWallpaper request: [{}]",
                                screen
                            );
                            response
                                .send(IPCResponse::Error(IPCError::ScreenNotFound))
                                .unwrap();
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
}
