use std::time::Duration;

use avian3d::prelude::SpatialQuery;
use bevy::{prelude::*, utils::hashbrown::HashSet};
use bevy_renet::netcode::NetcodeClientTransport;
use boxman_shared::{
    moveable_sim::{move_simulation, MoveableCorrectionState, MoveableSimulation, MoveableVisuals}, 
    character::{alter_character_velocity, LocalCharacterSimulation, CharacterSimulation, PLAYER_CONTROLLER_AIR_ACCEL, PLAYER_CONTROLLER_AIR_FRICTION, PLAYER_CONTROLLER_GROUND_ACCEL, PLAYER_CONTROLLER_GROUND_FRICTION, PLAYER_CONTROLLER_JUMP_IMPULSE, PLAYER_CONTROLLER_SPEED}, 
    snapshot::{CharacterSnapshotDiff, SnapshotDiff}
};

use crate::{config::MultiplayerConfig, player::InputHistory};

#[derive(Resource)]
pub struct LastProcessedSnapshotId(pub Option<u64>);

#[derive(Event)]
pub struct SnapshotDiffEvent(pub SnapshotDiff);

pub struct SnapshotPlugin;

impl Plugin for SnapshotPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LastProcessedSnapshotId(None));
        app.add_event::<SnapshotDiffEvent>();
        app.add_systems(
            FixedPostUpdate, 
            snapshot_system.run_if(resource_exists::<MultiplayerConfig>)
        );
    }
}

fn snapshot_system(
    cfg: Res<MultiplayerConfig>,
    spatial_query: SpatialQuery,
    mut last_processed_snapshot_id: ResMut<LastProcessedSnapshotId>,
    mut snapshot_diff_events: EventReader<SnapshotDiffEvent>,
    mut characters: Query<(Entity, &mut Transform, &CharacterSimulation, &mut MoveableSimulation), (Without<LocalCharacterSimulation>, Without<MoveableVisuals>)>,
    mut local_characters: Query<(Entity, &mut Transform, &mut MoveableSimulation), (With<LocalCharacterSimulation>, Without<MoveableVisuals>)>,
    transport: Option<Res<NetcodeClientTransport>>,
    fixed_time: Res<Time<Fixed>>,
    mut input_history: ResMut<InputHistory>,
) {
    if let Some(transport) = transport {
        let latest_snapshot = snapshot_diff_events.read()
            .max_by_key(|event| event.0.id);

        if let Some(event) = latest_snapshot {
            let snapshot_diff = &event.0;
            
            // Skip if we've already processed a newer snapshot
            if let Some(last_id) = last_processed_snapshot_id.0 {
                if snapshot_diff.id <= last_id {
                    return;
                }
            }
            
            last_processed_snapshot_id.0 = Some(snapshot_diff.id);
            
            for player_snapshot_diff in snapshot_diff.character_snapshots.iter() {
                let is_local = player_snapshot_diff.client_id == transport.client_id();

                if is_local {
                    reconcile_local_character(
                        &cfg,
                        &spatial_query,
                        &fixed_time,
                        &mut local_characters,
                        player_snapshot_diff,
                        &mut input_history,
                        snapshot_diff.acked_input_id,
                    );
                } else {
                    let existing_controller = characters.iter_mut()
                        .find(|(_, _, pc, _)| pc.client_id == player_snapshot_diff.client_id);
    
                    if let Some((_, mut transform, _, mut simulation)) = existing_controller {
                        if let Some(position) = player_snapshot_diff.position {
                            transform.translation = position;
                        }
                        if let Some(velocity) = player_snapshot_diff.velocity {
                            simulation.velocity = velocity;
                        }
                    }
                }
            }
        }
    }
}

fn reconcile_local_character(
    cfg: &MultiplayerConfig,
    spatial_query: &SpatialQuery,
    fixed_time: &Time<Fixed>,
    character_query: &mut Query<(Entity, &mut Transform, &mut MoveableSimulation), (With<LocalCharacterSimulation>, Without<MoveableVisuals>)>,
    snapshot: &CharacterSnapshotDiff,
    input_history: &mut InputHistory,
    acked_input_id: Option<u32>,
) {
    if let Ok((entity, mut transform, mut simulation)) = character_query.get_single_mut() {
        if let Some(position) = snapshot.position {
            if let Some(acked_input_id) = acked_input_id {
                let acked_input = input_history.inputs.iter().find(|input| input.id == acked_input_id);

                if let Some(acked_input) = acked_input {
                    let correction_distance = position.distance(acked_input.post_move_position);
                    
                    if correction_distance < cfg.moveable_correction_position_distance_threshold {
                        return;
                    }

                    simulation.is_visually_correcting = true;

                    if snapshot.velocity.is_none() {
                        simulation.velocity = acked_input.post_move_velocity;
                    }

                    if snapshot.grounded.is_none() {
                        simulation.grounded = acked_input.post_move_grounded;
                    }
                }

                transform.translation = position;

                if let Some(velocity) = snapshot.velocity {
                    simulation.velocity = velocity;
                }
                if let Some(grounded) = snapshot.grounded {
                    simulation.grounded = grounded;
                }

                let stored_rotation = transform.rotation;

                for input in input_history.inputs.iter_mut() {
                    if input.id <= acked_input_id {
                        continue;
                    }

                    alter_character_velocity(
                        &mut simulation, 
                        input, 
                        fixed_time.delta_secs(), 
                        PLAYER_CONTROLLER_SPEED, 
                        PLAYER_CONTROLLER_JUMP_IMPULSE, 
                        PLAYER_CONTROLLER_GROUND_ACCEL,
                        PLAYER_CONTROLLER_AIR_ACCEL,
                        PLAYER_CONTROLLER_GROUND_FRICTION, 
                        PLAYER_CONTROLLER_AIR_FRICTION,
                    );

                    move_simulation(
                        &fixed_time,
                        &spatial_query,
                        &mut simulation,
                        &mut transform,
                        entity
                    );
                    
                    input.post_move_velocity = simulation.velocity;
                    input.post_move_position = transform.translation;
                    input.post_move_grounded = simulation.grounded;
                }

                // Since we moved a bunch, its just safe to reset the rotation to the stored value.
                transform.rotation = stored_rotation;
            }
        }
    }
}
