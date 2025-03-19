use bevy::math::Vec3;
use serde::{Deserialize, Serialize};

use crate::{player::PlayerInput, snapshot::SnapshotDiff};

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerToClientMessage {
    PlayerJoined {
        id: u64,
        name: String,
    },
    SnapshotDiff(SnapshotDiff),
    SpawnCharacter {
        id: u64,
        position: Vec3,
        velocity: Vec3,
        grounded: bool,
    },
    DespawnCharacter {
        id: u64,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientToServerMessage {
    PlayerInput(PlayerInput),
}
