use serde::{Deserialize, Serialize};

use crate::{player::PlayerInput, snapshot::SnapshotDiff};

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerToClientMessage {
    PlayerJoined {
        id: u32,
        name: String,
    },
    SnapshotDiff(SnapshotDiff),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientToServerMessage {
    PlayerInput(PlayerInput),
}
