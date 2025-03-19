use bevy::prelude::*;
use bevy_renet::netcode::NetcodeClientTransport;
use boxman_shared::prelude::{CharacterSimulation, LocalCharacterSimulation};

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, 
            (
                tag_as_local_system,
                spawn_visuals_system
            )
        );
    }
}

/// Listens for new characters and tags them as local if they are ours.
fn tag_as_local_system(
    transport: Option<Res<NetcodeClientTransport>>,
    mut commands: Commands,
    characters: Query<(Entity, &CharacterSimulation)>,
) {
    let client_id = if let Some(transport) = transport {
        transport.client_id()
    } else {
        0
    };

    for (entity, character) in characters.iter() {
        if character.client_id == client_id {
            commands.entity(entity).insert(LocalCharacterSimulation);
        }
    }
}

/// Listens for new characters and spawns visuals for them.
fn spawn_visuals_system(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    characters: Query<Entity, Added<CharacterSimulation>>,
) {
    for entity in characters.iter() {
        commands.entity(entity).with_children(|parent| {
            parent.spawn(
                (
                    Mesh3d::from(meshes.add(Cuboid::default())),
                    MeshMaterial3d::from(materials.add(Color::srgb(0.9, 0.1, 0.1))),
                )
            );
        });
    }
}
