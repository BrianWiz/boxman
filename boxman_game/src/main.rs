mod moveable_vis;
mod player;
mod config;
mod net;

use avian3d::{prelude::ColliderConstructor, PhysicsPlugins};
use bevy::prelude::*;
use bevy_renet::renet::RenetServer;
use bevy_config_stack::prelude::*;
use boxman_server::{listen, player::{spawn_bot, BotIdTracker}};
use boxman_shared::{moveable_sim::MoveableSimulationPlugin, weapons::{WeaponsListConfig, WeaponsPlugin}};
use moveable_vis::MoveableVisualsPlugin;
use player::PlayerPlugin;
use clap::Parser;
use config::{MultiplayerConfig, ControlsConfig};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, name = "Boxman", author = "Riverside Games")]
pub struct CommandLineArgs {
    #[arg(long)]
    pub server: bool,

    #[arg(long, default_value = "127.0.0.1")]
    pub server_ip: String,

    #[arg(long, default_value_t = 5000)]
    pub port: u16,
}

#[derive(Resource)]
pub struct ServerIp(String);

#[derive(Resource)]
pub struct ServerPort(u16);

fn main() {
    let args = CommandLineArgs::parse();

    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        PhysicsPlugins::default(),
        ConfigAssetLoaderPlugin::<ControlsConfig>::new("config/controls.ron"),
        ConfigAssetLoaderPlugin::<MultiplayerConfig>::new("config/multiplayer.ron"),
        ConfigAssetLoaderPlugin::<WeaponsListConfig>::new("config/weapons.ron"),
        PlayerPlugin,
        MoveableSimulationPlugin,
        MoveableVisualsPlugin,
    ));

    app.insert_resource(ServerPort(args.port));

    // let default_weapons_list_config = boxman_shared::weapons::WeaponConfig::default();
    // let default_weapons_list_config_ron = ron::ser::to_string_pretty(&default_weapons_list_config, ron::ser::PrettyConfig::default()).unwrap();
    // println!("{}", default_weapons_list_config_ron);
    
    if args.server {
        app.add_plugins(boxman_server::BoxmanServerPlugin);
    } else {
        app.insert_resource(ServerIp(args.server_ip.clone()));
        app.add_plugins(net::BoxmanClientPlugin);
    }

    app.add_systems(Startup, (
        startup_system,
    ));

    app.add_plugins((
        WeaponsPlugin,
    ));

    app.run();
}

fn startup_system(
    renet_server: Option<Res<RenetServer>>,
    server_port: Res<ServerPort>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    mut bot_id_tracker: ResMut<BotIdTracker>,
) {
    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::default()
            .with_translation(Vec3::new(4.0, 4.0, 4.0)),
    ));

    // Floor
    commands.spawn((
        Mesh3d::from(meshes.add(Cuboid::new(40.0, 1.0, 40.0))),
        MeshMaterial3d::from(materials.add(Color::WHITE)),
        ColliderConstructor::ConvexHullFromMesh,
        Transform::default()
            .with_translation(Vec3::new(0.0, -1.0, 0.0)),
    ));

    // Box
    commands.spawn((
        Mesh3d::from(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d::from(materials.add(Color::srgb(0.5, 0.5, 0.5))),
        ColliderConstructor::ConvexHullFromMesh,
    ));

    // North wall
    commands.spawn((
        Mesh3d::from(meshes.add(Cuboid::new(40.0, 4.0, 1.0))),
        MeshMaterial3d::from(materials.add(Color::srgb(0.5, 0.5, 0.5))),
        ColliderConstructor::ConvexHullFromMesh,
        Transform::default()
            .with_translation(Vec3::new(0.0, 1.0, 20.0)),
    ));
    
    // South wall
    commands.spawn((
        Mesh3d::from(meshes.add(Cuboid::new(40.0, 4.0, 1.0))),
        MeshMaterial3d::from(materials.add(Color::srgb(0.5, 0.5, 0.5))),
        ColliderConstructor::ConvexHullFromMesh,
        Transform::default()
            .with_translation(Vec3::new(0.0, 1.0, -20.0)),
    ));

    // East wall 
    commands.spawn((
        Mesh3d::from(meshes.add(Cuboid::new(1.0, 4.0, 40.0))),
        MeshMaterial3d::from(materials.add(Color::srgb(0.5, 0.5, 0.5))),
        ColliderConstructor::ConvexHullFromMesh,
        Transform::default()
            .with_translation(Vec3::new(20.0, 1.0, 0.0)),
    ));

    // West wall
    commands.spawn((
        Mesh3d::from(meshes.add(Cuboid::new(1.0, 4.0, 40.0))),
        MeshMaterial3d::from(materials.add(Color::srgb(0.5, 0.5, 0.5))),
        ColliderConstructor::ConvexHullFromMesh, 
        Transform::default()
            .with_translation(Vec3::new(-20.0, 1.0, 0.0)),
    ));

    if renet_server.is_some() {
        if let Err(e) = listen(&mut commands, server_port.0) {
            error!("Failed to listen on port {}: {}", server_port.0, e);
        }

        spawn_bot(
            &mut bot_id_tracker,
            &mut commands, 
            &mut meshes, 
            &mut materials, 
            Vec3::new(0.0, 2.0, 0.0), 
        );
    }
}
