use bevy::prelude::*;

#[derive(Resource)]
pub struct GameServer;

#[derive(Resource)]
pub struct GameClient;

#[derive(Resource)]
pub struct ServerPort(pub u16);

#[derive(Resource)]
pub struct ServerIp(pub String);