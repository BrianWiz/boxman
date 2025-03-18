use bevy::prelude::*;
use bevy_renet::renet::ServerEvent;
use boxman_shared::{
    moveable_sim::MoveableSimulation, 
    player::{alter_player_controller_velocity, despawn_player_controller, spawn_player_controller, PlayerControllerSimulation, PlayerInput, PLAYER_CONTROLLER_AIR_ACCEL, PLAYER_CONTROLLER_AIR_FRICTION, PLAYER_CONTROLLER_GROUND_ACCEL, PLAYER_CONTROLLER_GROUND_FRICTION, PLAYER_CONTROLLER_JUMP_IMPULSE, PLAYER_CONTROLLER_SPEED}, weapons::Inventory
};
use rand::Rng;

use crate::bots::Bot;

#[derive(Component)]
pub struct Player {
    pub client_id: u64,
    pub name: String,
    pub last_acked_snapshot_id: Option<u64>,
    pub newest_processed_input_id: Option<u32>,
    pub last_input: Option<PlayerInput>,
}

#[derive(Component)]
pub struct PlayerInputQueue {
    pub inputs: Vec<PlayerInput>,
}

#[derive(Event)]
pub struct PlayerInputEvent(pub u64, pub PlayerInput);

#[derive(Event)]
pub struct DeletePlayerControllerEvent(pub u64);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlayerInputEvent>();
        app.add_event::<DeletePlayerControllerEvent>();
        app.add_systems(PostUpdate, (
            connection_event_receiver_system, 
            player_input_receiver_system,
        ));
        app.add_systems(FixedPreUpdate, (
            player_input_consumer_system, 
        ));
    }
}

fn connection_event_receiver_system(
    players: Query<(Entity, &Player)>,
    mut commands: Commands,
    mut server_events: EventReader<ServerEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_controllers: Query<(Entity, &MoveableSimulation, &PlayerControllerSimulation)>,
    mut delete_player_controller_events: EventWriter<DeletePlayerControllerEvent>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                info!("Player {} connected", client_id);
                commands.spawn((
                    Player {
                        client_id: *client_id,
                        name: format!("Player {}", client_id),
                        last_acked_snapshot_id: None,
                        newest_processed_input_id: None,
                        last_input: None,
                    },
                    PlayerInputQueue {
                        inputs: Vec::new(),
                    }
                ));

                // spawn a player controller
                // @todo: support headless mode
                spawn_player_controller(
                    &mut commands, 
                    Vec3::new(0.0, 2.0, 0.0), // position
                    *client_id, 
                    false, // is local
                    Some(&mut meshes), 
                    Some(&mut materials)
                );
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                for (entity, player) in players.iter() {
                    if player.client_id == *client_id {
                        info!("Player {} ({}) disconnected: {:?}", client_id, player.name, reason);
                        
                        // Remove the player
                        commands.entity(entity).despawn_recursive();

                        // Remove their controller
                        for (simulation_entity, simulation, player_controller) in player_controllers.iter() {
                            if player_controller.client_id == *client_id {
                                despawn_player_controller(&mut commands, simulation_entity, simulation.visuals());
                                break;
                            }
                        }

                        // Notify app that this player controller was deleted. 
                        // Things like the snapshot system will need to know about this.
                        delete_player_controller_events.send(DeletePlayerControllerEvent(*client_id));
                    }
                }
            }
        }
    }
}

fn player_input_receiver_system(
    mut player_input_events: EventReader<PlayerInputEvent>,
    mut players: Query<(&mut PlayerInputQueue, &mut Player)>,
) {
    for event in player_input_events.read() {        
        let matching_player = players.iter_mut()
            .find(|(_, player)| player.client_id == event.0);

        if let Some((mut input_queue, mut player)) = matching_player {
            // Don't process inputs older than what we've already processed
            // @todo-brian: What we really should be doing here is 
            // comparing to a small history of already consumed inputs.
            // if let Some(last_processed_id) = player.last_processed_input_id {
            //     if event.1.id <= last_processed_id {
            //         continue;
            //     }
            // }

            input_queue.inputs.push(event.1.clone());
            input_queue.inputs.sort_by_key(|input| input.id);
            input_queue.inputs.reverse();

            if let (Some(last_acked_snapshot_id), Some(pending_ack_snapshot_id)) = (player.last_acked_snapshot_id, event.1.snapshot_id) {
                if pending_ack_snapshot_id > last_acked_snapshot_id {
                    player.last_acked_snapshot_id = Some(pending_ack_snapshot_id);
                }
            } else {
                player.last_acked_snapshot_id = event.1.snapshot_id;
            }
        } else {
            warn!("No player found for client {}", event.0);
        }
    }
}

fn player_input_consumer_system(
    mut players: Query<(&mut PlayerInputQueue, &mut Player)>,
    mut player_controllers: Query<(&mut Inventory, &mut MoveableSimulation, &mut Transform, &PlayerControllerSimulation), Without<Bot>>,
    fixed_time: Res<Time<Fixed>>,
) {
    for (mut input_queue, mut player) in players.iter_mut() {
        let input = if !input_queue.inputs.is_empty() {
            let input = input_queue.inputs.remove(0);
            // Only update last_input if this is a newer input
            if let Some(last_input) = &player.last_input {
                if input.id > last_input.id {
                    player.last_input = Some(input.clone());
                }
            } else {
                player.last_input = Some(input.clone());
            }
            input
        } else if let Some(last_input) = &player.last_input {
            last_input.clone()
        } else {
            continue;
        };

        for (mut inventory, mut simulation, mut transform, controller) in player_controllers.iter_mut() {
            if controller.client_id == player.client_id {
                alter_player_controller_velocity(
                    &mut simulation,
                    &input,
                    fixed_time.delta_secs(),
                    PLAYER_CONTROLLER_SPEED,
                    PLAYER_CONTROLLER_JUMP_IMPULSE,
                    PLAYER_CONTROLLER_GROUND_ACCEL,
                    PLAYER_CONTROLLER_AIR_ACCEL,
                    PLAYER_CONTROLLER_GROUND_FRICTION,
                    PLAYER_CONTROLLER_AIR_FRICTION,
                );

                let active_weapon_key = inventory.active_weapon as usize;
                let weapon = &mut inventory.weapons[active_weapon_key];
                weapon.wish_fire = input.wish_fire;

                if let Some(last_id) = player.newest_processed_input_id {
                    if input.id > last_id {
                        player.newest_processed_input_id = Some(input.id);

                        // use the rotation only from the newest input
                        let (_, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
                        transform.rotation = Quat::from_euler(EulerRot::YXZ, input.yaw, pitch, roll);
                    }
                } else {
                    player.newest_processed_input_id = Some(input.id);

                    // use the rotation only from the newest input
                    let (_, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
                    transform.rotation = Quat::from_euler(EulerRot::YXZ, input.yaw, pitch, roll);
                }
            }
        }
    }
}
