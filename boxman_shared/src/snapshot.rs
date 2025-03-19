use bevy::prelude::*;

#[derive(Debug)]
pub struct Snapshot {
    pub id: u64,
    pub character_snapshots: Vec<CharacterSnapshot>,
}

impl Snapshot {
    pub fn diff(&self, other: &Self) -> SnapshotDiff {
        SnapshotDiff {
            id: self.id,
            acked_input_id: None, // Should be filled in after calling this function.
            character_snapshots: {
                let mut out = Vec::new();
                for controller in self.character_snapshots.iter() {
                    let other_controller = other.character_snapshots.iter().find(|c| c.client_id == controller.client_id);
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
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SnapshotDiff {
    pub id: u64,
    pub character_snapshots: Vec<CharacterSnapshotDiff>,
    pub acked_input_id: Option<u32>,
}

impl From<&Snapshot> for SnapshotDiff {
    fn from(snapshot: &Snapshot) -> Self {
        SnapshotDiff {
            id: snapshot.id,
            acked_input_id: None,
            character_snapshots: snapshot.character_snapshots.iter().map(|c| c.into()).collect(),
        }
    }
}

#[derive(Debug)]
pub struct CharacterSnapshot {
    pub client_id: u64,
    pub translation: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub grounded: bool,
}

impl CharacterSnapshot {
    pub fn diff(&self, other: &Self) -> Option<CharacterSnapshotDiff> {
        let out = CharacterSnapshotDiff {
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
pub struct CharacterSnapshotDiff {
    pub client_id: u64,
    pub position: Option<Vec3>,
    pub velocity: Option<Vec3>,
    pub yaw: Option<f32>,
    pub pitch: Option<f32>,
    pub grounded: Option<bool>,
}

impl From<&CharacterSnapshot> for CharacterSnapshotDiff     {
    fn from(snapshot: &CharacterSnapshot) -> Self {
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
