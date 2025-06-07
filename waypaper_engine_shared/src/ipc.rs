use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IPCRequest {
    SetWallpaper { id: u64, screen: String },
    KillDaemon,
}
