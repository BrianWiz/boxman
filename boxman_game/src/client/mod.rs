pub mod snapshot;

use std::{
    error::Error,
    net::{SocketAddr, UdpSocket},
    time::SystemTime,
};

use bevy::prelude::*;
use bevy_renet::{
    netcode::{ClientAuthentication, NetcodeClientPlugin, NetcodeClientTransport},
    renet::{ConnectionConfig, DefaultChannel, RenetClient},
    RenetClientPlugin,
};
use boxman_shared::{moveable_sim::MoveableSimulation, player::{despawn_character, spawn_character, CharacterSimulation, CharacterVisuals}, protocol::{ClientToServerMessage, ServerToClientMessage}, utils::Client};

use crate::{player::InputHistory, ServerIp, ServerPort};
use snapshot::{SnapshotDiffEvent, SnapshotPlugin};

pub struct GameClientPlugin;

impl Plugin for GameClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RenetClientPlugin, 
            NetcodeClientPlugin, 
            SnapshotPlugin
        ));
        app.insert_resource(Client);
        app.add_systems(Startup, startup_system);
        app.add_systems(Update, (
            message_receiver_system.run_if(resource_exists::<RenetClient>),
            send_input_system.run_if(resource_exists::<RenetClient>)
        ));
    }
}

fn startup_system(mut commands: Commands, server_ip: Res<ServerIp>, server_port: Res<ServerPort>) {
    if let Err(e) = connect_to_server(&mut commands, &server_ip, &server_port) {
        error!("Failed to connect to server: {}", e);
    }
}

pub fn connect_to_server(
    commands: &mut Commands,
    server_ip: &ServerIp,
    server_port: &ServerPort,
) -> Result<(), Box<dyn Error>> {
    info!("Connecting to server at {}", server_ip.0);
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let authentication = ClientAuthentication::Unsecure {
        server_addr: SocketAddr::new(server_ip.0.parse()?, server_port.0),
        user_data: None,
        protocol_id: 0,
        client_id: current_time.as_millis() as u64,
    };

    let socket = UdpSocket::bind("127.0.0.1:0")?;
    let transport = NetcodeClientTransport::new(current_time, authentication, socket)?;
    let client = RenetClient::new(ConnectionConfig::default());
    commands.insert_resource(transport);
    commands.insert_resource(client);
    Ok(())
}

pub fn message_receiver_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut transport: ResMut<NetcodeClientTransport>,
    mut renet_client: ResMut<RenetClient>,
    mut snapshot_diff_events: EventWriter<SnapshotDiffEvent>,
    mut characters: Query<(Entity, &CharacterSimulation, &MoveableSimulation)>,
) {
    let my_id = transport.client_id();

    while let Some(message) = renet_client.receive_message(DefaultChannel::Unreliable) {
        match bincode::deserialize::<ServerToClientMessage>(&message) {
            Ok(ServerToClientMessage::SnapshotDiff(snapshot_diff)) => {
                snapshot_diff_events.send(SnapshotDiffEvent(snapshot_diff));
            }
            Ok(ServerToClientMessage::PlayerJoined { id, name }) => {
                info!("Player joined: {} {}", id, name);
            }
            Ok(_) => {
                error!("Received unknown message from server on unreliable channel");
            }
            Err(e) => {
                error!("Failed to deserialize message: {}", e);
            }
        }
    }

    while let Some(message) = renet_client.receive_message(DefaultChannel::ReliableOrdered) {
        match bincode::deserialize::<ServerToClientMessage>(&message) {
            Ok(ServerToClientMessage::SpawnCharacter { id, position, velocity, grounded }) => {
                info!("Spawned character: {}", id);
                spawn_character(
                    &mut commands, 
                    position, 
                    id, 
                    id == my_id,
                    Some(&mut meshes), 
                    Some(&mut materials)
                );
            }
            Ok(ServerToClientMessage::DespawnCharacter { id }) => {
                for (entity, character_simulation, simulation) in characters.iter_mut() {
                    if character_simulation.client_id == id {
                        info!("Despawned character: {}", id);
                        despawn_character(&mut commands, entity, simulation.visuals());
                        break;
                    }
                }
            }
            Ok(_) => {
                error!("Received unknown message from server on reliable channel");
            }
            Err(e) => {
                error!("Failed to deserialize message: {}", e);
            }
        }
    }
}

pub fn send_input_system(
    client: Option<ResMut<RenetClient>>,
    player_inputs: Option<ResMut<InputHistory>>,
) {
    if let Some(mut client) = client {
        if let Some(mut player_inputs) = player_inputs {
            for input in player_inputs.inputs.iter_mut() {
                if input.send_count > 0 {
                    continue;
                }
                match bincode::serialize(&ClientToServerMessage::PlayerInput(input.clone())) {
                    Ok(serialized) => {
                        client.send_message(DefaultChannel::Unreliable, serialized);
                        input.send_count = 1;
                    }
                    Err(e) => {
                        error!("Failed to serialize input: {}", e);
                    }
                }
            }
        }
    }
}
