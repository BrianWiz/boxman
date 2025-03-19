use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetServer, ServerEvent};
use boxman_shared::{
    character::{alter_character_velocity, CharacterSimulation, PlayerInput, PLAYER_CONTROLLER_AIR_ACCEL, PLAYER_CONTROLLER_AIR_FRICTION, PLAYER_CONTROLLER_GROUND_ACCEL, PLAYER_CONTROLLER_GROUND_FRICTION, PLAYER_CONTROLLER_JUMP_IMPULSE, PLAYER_CONTROLLER_SPEED}, moveable_sim::MoveableSimulation, prelude::{CharacterDespawnEvent, CharacterSpawnEvent, ServerToClientMessage}
};

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

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlayerInputEvent>();
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
    mut renet_server: ResMut<RenetServer>,
    mut server_events: EventReader<ServerEvent>,
    mut character_spawn_events: EventWriter<CharacterSpawnEvent>,
    mut character_despawn_events: EventWriter<CharacterDespawnEvent>,
    characters: Query<(Entity, &Transform, &CharacterSimulation)>,
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

                // get every character and tell the new client to spawn it
                for (_, transform, character_simulation) in characters.iter() {
                    let message = ServerToClientMessage::SpawnCharacter(CharacterSpawnEvent {
                        client_id: character_simulation.client_id,
                        position: transform.translation,
                        yaw: 0.0,
                    });

                    match bincode::serialize(&message) {
                        Ok(serialized) => {
                            renet_server.send_message(*client_id, DefaultChannel::ReliableOrdered, serialized);
                        }
                        Err(e) => {
                            error!("Error serializing message: {}", e);
                        }
                    }
                }

                // spawn their character
                let character_spawn_event = CharacterSpawnEvent {
                    client_id: *client_id,
                    position: Vec3::new(0.0, 2.0, 0.0),
                    yaw: 0.0,
                };
                character_spawn_events.send(character_spawn_event.clone());

                // tell every client about the new character
                for client_id in renet_server.clients_id() {
                    let message = ServerToClientMessage::SpawnCharacter(character_spawn_event.clone());

                    match bincode::serialize(&message) {
                        Ok(serialized) => {
                            renet_server.send_message(client_id, DefaultChannel::ReliableOrdered, serialized);
                        }
                        Err(e) => {
                            error!("Error serializing message: {}", e);
                        }
                    }
                }
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                for (entity, player) in players.iter() {
                    if player.client_id == *client_id {
                        info!("Player {} ({}) disconnected: {:?}", client_id, player.name, reason);
                        commands.entity(entity).despawn_recursive();
        
                        // despawn the character
                        let character_despawn_event = CharacterDespawnEvent {
                            client_id: *client_id,
                        };
                        character_despawn_events.send(character_despawn_event.clone());
        
                        // tell everyone to despawn the character
                        for client_id in renet_server.clients_id() {
                            let message = ServerToClientMessage::DespawnCharacter(character_despawn_event.clone());
        
                            match bincode::serialize(&message) {
                                Ok(serialized) => {
                                    renet_server.send_message(client_id, DefaultChannel::ReliableOrdered, serialized);
                                }
                                Err(e) => {
                                    error!("Error serializing message: {}", e);
                                }
                            }
                        }
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
    mut player_controllers: Query<(&mut MoveableSimulation, &mut Transform, &CharacterSimulation)>,
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

        for (mut simulation, mut transform, controller) in player_controllers.iter_mut() {
            if controller.client_id == player.client_id {
                alter_character_velocity(
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
