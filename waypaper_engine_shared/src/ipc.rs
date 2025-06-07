use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IPCRequest {
    SetWallpaper { id: u64, screen: String },
    KillDaemon,
    ListOutputs,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IPCResponse {
    Success,
    Outputs(Vec<String>),
    Error(IPCError)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IPCError {
    ScreenNotFound,
    WallpaperNotFound,
    UnsupportedWallpaperType,
    InternalError,
}

