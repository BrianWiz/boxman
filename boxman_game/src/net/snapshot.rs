use std::time::Duration;

use avian3d::prelude::SpatialQuery;
use bevy::{prelude::*, utils::hashbrown::HashSet};
use bevy_renet::netcode::NetcodeClientTransport;
use boxman_shared::{
    moveable_sim::{move_simulation, MoveableCorrectionState, MoveableSimulation, MoveableVisuals}, 
    player::{alter_player_controller_velocity, despawn_player_controller, spawn_player_controller, LocalPlayerControllerSimulation, PlayerControllerSimulation, PLAYER_CONTROLLER_AIR_ACCEL, PLAYER_CONTROLLER_AIR_FRICTION, PLAYER_CONTROLLER_GROUND_ACCEL, PLAYER_CONTROLLER_GROUND_FRICTION, PLAYER_CONTROLLER_JUMP_IMPULSE, PLAYER_CONTROLLER_SPEED}, 
    snapshot::{PlayerControllerSnapshotDiff, SnapshotDiff}
};

use crate::{config::MultiplayerConfig, player::PlayerControllerInputHistory};

#[derive(Resource)]
pub struct ReservedPlayerControllerIds(pub HashSet<u64>);

#[derive(Resource)]
pub struct DeletedPlayerControllerIds(pub HashSet<u64>);

#[derive(Resource)]
pub struct LastProcessedSnapshotId(pub Option<u64>);

#[derive(Event)]
pub struct SnapshotDiffEvent(pub SnapshotDiff);

pub struct SnapshotPlugin;

impl Plugin for SnapshotPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LastProcessedSnapshotId(None));
        app.insert_resource(ReservedPlayerControllerIds(HashSet::new()));
        app.insert_resource(DeletedPlayerControllerIds(HashSet::new()));
        app.add_event::<SnapshotDiffEvent>();
        app.add_systems(
            FixedPostUpdate, 
            snapshot_system.run_if(resource_exists::<MultiplayerConfig>)
        );
    }
}

fn snapshot_system(
    cfg: Res<MultiplayerConfig>,
    mut commands: Commands,
    spatial_query: SpatialQuery,
    mut last_processed_snapshot_id: ResMut<LastProcessedSnapshotId>,
    mut snapshot_diff_events: EventReader<SnapshotDiffEvent>,
    mut player_controllers: Query<(Entity, &mut Transform, &PlayerControllerSimulation, &mut MoveableSimulation), (Without<LocalPlayerControllerSimulation>, Without<MoveableVisuals>)>,
    mut local_player_controllers: Query<(Entity, &mut Transform, &mut MoveableSimulation), (With<LocalPlayerControllerSimulation>, Without<MoveableVisuals>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut reserved_player_controller_ids: ResMut<ReservedPlayerControllerIds>,
    mut deleted_player_controller_ids: ResMut<DeletedPlayerControllerIds>,
    transport: Option<Res<NetcodeClientTransport>>,
    fixed_time: Res<Time<Fixed>>,
    mut player_inputs: ResMut<PlayerControllerInputHistory>,
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
            
            for player_snapshot_diff in snapshot_diff.controllers.iter() {
                let is_local = player_snapshot_diff.client_id == transport.client_id();

                if is_local {
                    reconcile_local_player_controller(
                        &cfg,
                        &spatial_query,
                        &fixed_time,
                        &mut local_player_controllers,
                        player_snapshot_diff,
                        &mut player_inputs,
                        snapshot_diff.acked_input_id,
                    );
                }
                
                let existing_controller = player_controllers.iter_mut()
                    .find(|(_, _, pc, _)| pc.client_id == player_snapshot_diff.client_id);

                if let Some((_, mut transform, _, mut simulation)) = existing_controller {
                    if let Some(position) = player_snapshot_diff.position {
                        transform.translation = position;
                    }
                    if let Some(velocity) = player_snapshot_diff.velocity {
                        simulation.velocity = velocity;
                    }
                } else {
                    if reserved_player_controller_ids.0.contains(&player_snapshot_diff.client_id) {
                        continue;
                    }
                    if deleted_player_controller_ids.0.contains(&player_snapshot_diff.client_id) {
                        continue;
                    }
                    if let Some(position) = player_snapshot_diff.position {
                        spawn_player_controller(
                            &mut commands,
                            position,
                            player_snapshot_diff.client_id,
                            is_local,
                            Some(&mut meshes),
                            Some(&mut materials),
                        );
                        reserved_player_controller_ids.0.insert(player_snapshot_diff.client_id);
                    }
                }
            }
            
            for deleted_player_controller_id in snapshot_diff.player_controller_deletions.iter() {
                deleted_player_controller_ids.0.insert(*deleted_player_controller_id);
                for (entity, _, player_controller, simulation) in player_controllers.iter_mut() {
                    if player_controller.client_id == *deleted_player_controller_id {
                        despawn_player_controller(&mut commands, entity, simulation.visuals());
                        break;
                    }
                }
            }
        }
    }
}

fn reconcile_local_player_controller(
    cfg: &MultiplayerConfig,
    spatial_query: &SpatialQuery,
    fixed_time: &Time<Fixed>,
    player_controller_query: &mut Query<(Entity, &mut Transform, &mut MoveableSimulation), (With<LocalPlayerControllerSimulation>, Without<MoveableVisuals>)>,
    snapshot: &PlayerControllerSnapshotDiff,
    player_inputs: &mut PlayerControllerInputHistory,
    acked_input_id: Option<u32>,
) {
    if let Ok((entity, mut transform, mut simulation)) = player_controller_query.get_single_mut() {
        if let Some(position) = snapshot.position {
            if let Some(acked_input_id) = acked_input_id {
                let acked_input = player_inputs.inputs.iter().find(|input| input.id == acked_input_id);

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

                for input in player_inputs.inputs.iter_mut() {
                    if input.id <= acked_input_id {
                        continue;
                    }

                    alter_player_controller_velocity(
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
