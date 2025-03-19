pub mod player;
pub mod bots;
mod snapshot;
use std::{error::Error, net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket}, time::SystemTime};

use bevy::prelude::*;
use bevy_renet::{
    netcode::{NetcodeServerPlugin, NetcodeServerTransport, ServerAuthentication, ServerConfig}, 
    renet::{ConnectionConfig, DefaultChannel, RenetServer, ServerEvent}, 
    RenetServerPlugin
};
use bots::BotsPlugin;
use boxman_shared::{moveable_sim::MoveableSimulation, player::{despawn_character, spawn_character, CharacterSimulation}, protocol::{ClientToServerMessage, ServerToClientMessage}, utils::GameServer};
use player::{PlayerInputEvent, PlayerPlugin};
use snapshot::SnapshotPlugin;

pub struct GameServerPlugin;

impl Plugin for GameServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RenetServerPlugin,
            NetcodeServerPlugin,
            SnapshotPlugin,
            PlayerPlugin,
            BotsPlugin,
        ));

        let server = RenetServer::new(ConnectionConfig::default());
        app.insert_resource(server);
        app.insert_resource(GameServer);
        app.add_systems(Update, (
            message_receiver_system,
            handle_connection_events_system
        ));
    }
}

fn message_receiver_system(
    mut renet_server: ResMut<RenetServer>,
    mut player_input_events: EventWriter<PlayerInputEvent>,
) {
    for client_id in renet_server.clients_id() {
        while let Some(message) = renet_server.receive_message(client_id, DefaultChannel::Unreliable) {
            match bincode::deserialize::<ClientToServerMessage>(&message) {
                Ok(ClientToServerMessage::PlayerInput(player_input)) => {
                    player_input_events.send(PlayerInputEvent(client_id, player_input));
                }
                Err(e) => {
                    error!("Error deserializing message from client {}: {}", client_id, e);
                }
            }
        }
    }
}

fn handle_connection_events_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut renet_server: ResMut<RenetServer>,
    mut server_events: EventReader<ServerEvent>,
    characters: Query<(Entity, &Transform, &CharacterSimulation, &MoveableSimulation)>
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                info!("Client {client_id} connected");

                // get every character and tell the new client to spawn it
                for (_, transform, character_simulation, moveable_simulation) in characters.iter() {
                    let message = ServerToClientMessage::SpawnCharacter {
                        id: character_simulation.client_id,
                        position: transform.translation,
                        velocity: moveable_simulation.velocity,
                        grounded: moveable_simulation.grounded,
                    };

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
                spawn_character(
                    &mut commands, 
                    Vec3::new(0.0, 2.0, 0.0), 
                    *client_id, 
                    false, 
                    Some(&mut meshes), 
                    Some(&mut materials)
                );

                // tell every client about the new character
                for client_id in renet_server.clients_id() {
                    let message = ServerToClientMessage::SpawnCharacter {
                        id: client_id,
                        position: Vec3::new(0.0, 2.0, 0.0),
                        velocity: Vec3::ZERO,
                        grounded: false,
                    };

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
                info!("Client {client_id} disconnected: {reason}");

                // tell everyone to despawn their character
                for client_id in renet_server.clients_id() {
                    let message = ServerToClientMessage::DespawnCharacter { id: client_id };

                    match bincode::serialize(&message) {
                        Ok(serialized) => {
                            renet_server.send_message(client_id, DefaultChannel::ReliableOrdered, serialized);
                        }
                        Err(e) => {
                            error!("Error serializing message: {}", e);
                        }
                    }
                }

                // delete the character
                for (entity, _, character_simulation, moveable_simulation) in characters.iter() {
                    if character_simulation.client_id == *client_id {
                        despawn_character(&mut commands, entity, moveable_simulation.visuals());
                        break;
                    }
                }
            }
        }
    }
}

pub fn listen(
    commands: &mut Commands,
    port: u16,
) -> Result<(), Box<dyn Error>> {
    let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
    let socket = UdpSocket::bind(socket_addr)?;
    let server_config = ServerConfig {
        current_time: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?,
        max_clients: 64,
        protocol_id: 0,
        public_addresses: vec![socket_addr],
        authentication: ServerAuthentication::Unsecure
    };
    let transport = NetcodeServerTransport::new(server_config, socket)?;
    commands.insert_resource(transport);
    info!("Listening on port {}", port);
    Ok(())
}
