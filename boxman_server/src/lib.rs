pub mod player;
mod snapshot;
use std::{error::Error, net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket}, time::SystemTime};

use bevy::prelude::*;
use bevy_renet::{
    netcode::{NetcodeServerPlugin, NetcodeServerTransport, ServerAuthentication, ServerConfig}, 
    renet::{ConnectionConfig, DefaultChannel, RenetServer}, 
    RenetServerPlugin
};
use boxman_shared::{protocol::ClientToServerMessage, utils::Server};
use player::{PlayerInputEvent, PlayerPlugin};
use snapshot::SnapshotPlugin;

pub struct BoxmanServerPlugin;

impl Plugin for BoxmanServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RenetServerPlugin,
            NetcodeServerPlugin,
            SnapshotPlugin,
            PlayerPlugin,
        ));

        let server = RenetServer::new(ConnectionConfig::default());
        app.insert_resource(server);
        app.insert_resource(Server);
        app.add_systems(Update, message_receiver_system);
    }
}

fn message_receiver_system(
    mut server: ResMut<RenetServer>,
    mut player_input_events: EventWriter<PlayerInputEvent>,
) {
    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, DefaultChannel::Unreliable) {
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
