use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IPCRequest {
    SetWP { id: u64, screen: String },
    StopDaemon,
}
