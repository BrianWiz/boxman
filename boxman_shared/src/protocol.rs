use bevy::math::Vec3;
use serde::{Deserialize, Serialize};

use crate::{character::{PlayerInput, CharacterDespawnEvent, CharacterSpawnEvent}, snapshot::SnapshotDiff};

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerToClientMessage {
    PlayerJoined {
        id: u64,
        name: String,
    },
    SnapshotDiff(SnapshotDiff),
    SpawnCharacter(CharacterSpawnEvent),
    DespawnCharacter(CharacterDespawnEvent),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientToServerMessage {
    PlayerInput(PlayerInput),
}
