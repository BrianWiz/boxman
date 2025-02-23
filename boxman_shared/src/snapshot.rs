use bevy::prelude::*;

#[derive(Debug)]
pub struct Snapshot {
    pub id: u64,
    pub controllers: Vec<PlayerControllerSnapshot>,
    pub player_controller_deletions: Vec<u64>,
}

impl Snapshot {
    pub fn diff(&self, other: &Self, deleted_player_controller_ids: &Vec<u64>) -> SnapshotDiff {
        SnapshotDiff {
            id: self.id,
            acked_input_id: None, // Should be filled in after calling this function.
            controllers: {
                let mut out = Vec::new();
                for controller in self.controllers.iter() {
                    let other_controller = other.controllers.iter().find(|c| c.client_id == controller.client_id);
                    if let Some(other_controller) = other_controller {
                        if let Some(diff) = controller.diff(other_controller) {
                            out.push(diff);
                        }
                    } else {
                        out.push(controller.into());
                    }
                }
                out
            },
            player_controller_deletions: deleted_player_controller_ids.clone(),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SnapshotDiff {
    pub id: u64,
    pub controllers: Vec<PlayerControllerSnapshotDiff>,
    pub acked_input_id: Option<u32>,
    pub player_controller_deletions: Vec<u64>,
}

impl From<&Snapshot> for SnapshotDiff {
    fn from(snapshot: &Snapshot) -> Self {
        SnapshotDiff {
            id: snapshot.id,
            acked_input_id: None,
            controllers: snapshot.controllers.iter().map(|c| c.into()).collect(),
            player_controller_deletions: snapshot.player_controller_deletions.clone(),
        }
    }
}

#[derive(Debug)]
pub struct PlayerControllerSnapshot {
    pub client_id: u64,
    pub translation: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub grounded: bool,
}

impl PlayerControllerSnapshot {
    pub fn diff(&self, other: &Self) -> Option<PlayerControllerSnapshotDiff> {
        let out = PlayerControllerSnapshotDiff {
            client_id: self.client_id,
            position: if self.translation != other.translation {
                Some(self.translation)
            } else {
                None
            },
            velocity: if self.velocity != other.velocity {
                Some(self.velocity)
            } else {
                None
            },
            yaw: if self.yaw != other.yaw {
                Some(self.yaw)
            } else {
                None
            },
            pitch: if self.pitch != other.pitch {
                Some(self.pitch)
            } else {
                None
            },
            grounded: if self.grounded != other.grounded {
                Some(self.grounded)
            } else {
                None
            },
        };

        if out.position.is_some() 
            || out.velocity.is_some() 
            || out.yaw.is_some() 
            || out.pitch.is_some() 
            || out.grounded.is_some() {
            Some(out)
        } else {
            None
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PlayerControllerSnapshotDiff {
    pub client_id: u64,
    pub position: Option<Vec3>,
    pub velocity: Option<Vec3>,
    pub yaw: Option<f32>,
    pub pitch: Option<f32>,
    pub grounded: Option<bool>,
}

impl From<&PlayerControllerSnapshot> for PlayerControllerSnapshotDiff     {
    fn from(snapshot: &PlayerControllerSnapshot) -> Self {
        Self {
            client_id: snapshot.client_id,
            position: Some(snapshot.translation),
            velocity: Some(snapshot.velocity),
            yaw: Some(snapshot.yaw),
            pitch: Some(snapshot.pitch),
            grounded: Some(snapshot.grounded),
        }
    }
}
