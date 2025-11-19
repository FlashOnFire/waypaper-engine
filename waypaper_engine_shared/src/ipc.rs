use serde::{Deserialize, Serialize};
use subenum::subenum;

#[subenum(IPCRequest)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InternalRequest {
    #[subenum(IPCRequest)]
    SetWallpaper { id: u64, screen: String },
    #[subenum(IPCRequest)]
    KillDaemon,
    #[subenum(IPCRequest)]
    ListOutputs,
    
    NewOutput { screen: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IPCResponse {
    Success,
    Outputs(Vec<String>),
    Error(IPCError),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IPCError {
    ScreenNotFound,
    WallpaperNotFound,
    UnsupportedWallpaperType,
    InternalError,
    WallpaperLoadingError
}
