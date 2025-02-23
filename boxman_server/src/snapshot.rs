use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetServer};
use boxman_shared::{moveable_sim::MoveableSimulation, player::PlayerControllerSimulation, snapshot::{PlayerControllerSnapshot, Snapshot, SnapshotDiff}};
use boxman_shared::protocol::ServerToClientMessage;

use crate::player::Player;

#[derive(Resource)]
pub struct SnapshotContainer {
    pub next_id: u64,
    pub snapshots: Vec<Snapshot>,
}

pub struct SnapshotPlugin;

impl Plugin for SnapshotPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SnapshotContainer {
            next_id: 0,
            snapshots: Vec::new(),
        });
        app.add_systems(FixedPostUpdate, 
            (
                snapshot_system,
                send_snapshot_diff_system,
            )
            .chain()
        );
    }
}

fn snapshot_system(
    mut snapshot_container: ResMut<SnapshotContainer>,
    player_controllers: Query<(&PlayerControllerSimulation, &Transform, &MoveableSimulation)>,
) {
    let id = snapshot_container.next_id;
    snapshot_container.snapshots.push(Snapshot {
        id,
        controllers: {
            let mut controllers = Vec::new();
            for (player_controller, transform, moveable_simulation) in player_controllers.iter() {
                controllers.push(PlayerControllerSnapshot {
                    client_id: player_controller.client_id,
                    translation: transform.translation,
                    velocity: moveable_simulation.velocity,
                    yaw: transform.rotation.to_euler(EulerRot::YXZ).0,
                    pitch: 0.0,
                    grounded: moveable_simulation.grounded,
                });
            }
            controllers
        },
        player_controller_deletions: Vec::new(),
    });
    snapshot_container.next_id += 1;

    // keep the last 64 snapshots (1 second of snapshots)
    if snapshot_container.snapshots.len() > 64 {
        snapshot_container.snapshots.remove(0);
    }
}

fn send_snapshot_diff_system(
    snapshot_container: Res<SnapshotContainer>,
    mut server: ResMut<RenetServer>,
    players: Query<&Player>,
) {
    if snapshot_container.snapshots.is_empty() {
        return;
    }

    let latest_snapshot = snapshot_container.snapshots.last().unwrap();
    for client_id in server.clients_id() {
        if let Some(player) = players.iter().find(|p| p.client_id == client_id) {
            let last_acked_snapshot = if let Some(last_acked_snapshot_id) = player.last_acked_snapshot_id {
                snapshot_container.snapshots.iter().find(|s| s.id == last_acked_snapshot_id)
            } else {
                None
            };

            if let Some(last_acked_snapshot) = last_acked_snapshot {
                let mut snapshot_diff = latest_snapshot.diff(last_acked_snapshot);
                snapshot_diff.acked_input_id = player.newest_processed_input_id;
                match bincode::serialize(&ServerToClientMessage::SnapshotDiff(snapshot_diff)) {
                    Ok(serialized) => {
                        server.send_message(client_id, DefaultChannel::Unreliable, serialized);
                    }
                    Err(e) => {
                        error!("Error serializing snapshot diff: {}", e);
                    }
                }
            } else {
                let mut snapshot_diff = SnapshotDiff::from(latest_snapshot);
                snapshot_diff.acked_input_id = player.newest_processed_input_id;
                match bincode::serialize(&ServerToClientMessage::SnapshotDiff(snapshot_diff)) {
                    Ok(serialized) => {
                        server.send_message(client_id, DefaultChannel::Unreliable, serialized);
                    }
                    Err(e) => {
                        error!("Error serializing snapshot: {}", e);
                    }
                }
            }
        }
    }
}
