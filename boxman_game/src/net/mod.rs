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
use boxman_shared::protocol::{ClientToServerMessage, ServerToClientMessage};

use crate::{player::PlayerControllerInputHistory, ServerIp, ServerPort};
use snapshot::{SnapshotDiffEvent, SnapshotPlugin};

pub struct BoxmanClientPlugin;

impl Plugin for BoxmanClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RenetClientPlugin, 
            NetcodeClientPlugin, 
            SnapshotPlugin
        ));
        app.add_systems(Startup, startup_system);
        app.add_systems(Update, (
            message_receiver_system,
            send_input_system
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
    client: Option<ResMut<RenetClient>>,
    mut snapshot_diff_events: EventWriter<SnapshotDiffEvent>,
) {
    if let Some(mut client) = client {
        while let Some(message) = client.receive_message(DefaultChannel::Unreliable) {
            match bincode::deserialize::<ServerToClientMessage>(&message) {
                Ok(ServerToClientMessage::SnapshotDiff(snapshot_diff)) => {
                    snapshot_diff_events.send(SnapshotDiffEvent(snapshot_diff));
                }
                Ok(ServerToClientMessage::PlayerJoined { id, name }) => {
                    info!("Player joined: {} {}", id, name);
                }
                Err(e) => {
                    error!("Failed to deserialize message: {}", e);
                }
            }
        }
    }
}

pub fn send_input_system(
    client: Option<ResMut<RenetClient>>,
    player_inputs: Option<ResMut<PlayerControllerInputHistory>>,
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
