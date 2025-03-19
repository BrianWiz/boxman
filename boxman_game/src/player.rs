use bevy::{prelude::*, window::PrimaryWindow};
use bevy_config_stack::prelude::ConfigAssetLoaderPlugin;
use bevy_renet::netcode::NetcodeClientTransport;
use boxman_shared::{character::{alter_character_velocity, LocalCharacter, LocalCharacterVisuals, PlayerInput}, data::CharacterConfig, moveable_sim::MoveableSimulation, prelude::{Character, CharacterVisuals, MoveableVisuals}};

use crate::client::snapshot::LastProcessedSnapshotId;
use boxman_shared::data::ControlsConfig;

const CAMERA_Y_OFFSET: f32 = 10.0;

#[derive(Resource)]
pub struct InputHistory {
    pub next_input_id: u32,
    pub inputs: Vec<PlayerInput>,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ConfigAssetLoaderPlugin::<ControlsConfig>::new("data/controls.ron"));
        app.add_systems(Startup, spawn_camera_system);
        app.add_systems(FixedPreUpdate, 
            (
                input_capture_system
                    .run_if(resource_exists::<ControlsConfig>),
                alter_velocity_system
            )
            .chain()
        );
        app.add_systems(FixedPostUpdate, post_move_system);
        app.add_systems(PostUpdate,(
            tag_as_local_system,
            spawn_visuals_system,
            camera_follow_system
        ));
        app.insert_resource(InputHistory {
            next_input_id: 0,
            inputs: Vec::new(),
        });
    }
}

fn input_capture_system(
    time: Res<Time<Fixed>>,
    mut input_history: ResMut<InputHistory>,
    snapshot_id: Option<ResMut<LastProcessedSnapshotId>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_controller: Query<(Entity, &Transform, &MoveableSimulation), With<LocalCharacter>>,
) {
    // We always send an input to the server regardless of whether we have a player controller or not.
    // So always create an input history entry.
    let player_controller = player_controller.get_single_mut();
    let wish_fire = mouse_input.pressed(MouseButton::Left);
    let id = input_history.next_input_id;
    input_history.inputs.push(PlayerInput {
        id,
        snapshot_id: if let Some(snapshot_id) = snapshot_id {
            Some(snapshot_id.0.unwrap_or_default())
        } else {
            None
        },
        yaw: { 
            if let Ok((_, player_transform, _)) = player_controller {
                player_transform.rotation.to_euler(EulerRot::YXZ).0
            } else {
                0.0
            }
        },
        wish_dir: {
            let mut direction = Vec2::ZERO;
            if keyboard_input.pressed(KeyCode::KeyW) {
                direction += Vec2::NEG_Y; // Move up on screen (negative Z)
            }
            if keyboard_input.pressed(KeyCode::KeyS) {
                direction += Vec2::Y; // Move down on screen (positive Z)
            }
            if keyboard_input.pressed(KeyCode::KeyA) {
                direction += Vec2::NEG_X; // Move left on screen (negative X)
            }
            if keyboard_input.pressed(KeyCode::KeyD) {
                direction += Vec2::X; // Move right on screen (positive X)
            }
            direction.normalize_or_zero()
        },
        wish_jump: {
            if let Ok(( _, _, moveable_simulation)) = player_controller {
                keyboard_input.pressed(KeyCode::Space) && moveable_simulation.grounded
            } else {
                false
            }
        },
        active_weapon: 0,
        wish_fire,
        send_count: 0,
        timestamp: time.elapsed_secs(),
        post_move_velocity: Vec3::ZERO,
        post_move_position: Vec3::ZERO,
        post_move_grounded: false,
    });
    input_history.next_input_id += 1;

    // Keep up to a second of input history, because we play these back when receiving a snapshot
    if input_history.inputs.len() > 64 {
        input_history.inputs.remove(0);
    }
}

fn alter_velocity_system(
    fixed_time: Res<Time<Fixed>>,
    mut characters: Query<&mut MoveableSimulation, (With<LocalCharacter>, Without<Camera3d>)>,
    mut player_inputs: ResMut<InputHistory>,
    character_config: Res<CharacterConfig>,
) {
    if let Ok(mut character) = characters.get_single_mut() {
        if let Some(input) = player_inputs.inputs.last_mut() {
            alter_character_velocity(
                &mut character, 
                input, 
                fixed_time.delta_secs(), 
                character_config.speed, 
                character_config.acceleration,
                character_config.friction, 
            );
        }
    }
}

fn post_move_system(
    mut player_inputs: ResMut<InputHistory>,
    player_controller: Query<(&MoveableSimulation, &Transform), With<LocalCharacter>>,
) {
    // We log these and store them on the input so that when we receive a snapshot,
    // we can compare the post-move values to the values in the snapshot to determine
    // if we should correct the client's movement.
    if let Some(input) = player_inputs.inputs.last_mut() {
        if let Ok((player_controller, player_transform)) = player_controller.get_single() {
            input.post_move_velocity = player_controller.velocity;
            input.post_move_position = player_transform.translation;
            input.post_move_grounded = player_controller.grounded;
        }
    }
}

fn spawn_camera_system(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 90.0f32.to_radians(),
            ..default()
        }),
        // top down camera at 0,0 rotated 90 degrees clockwise
        Transform::default()
            .with_translation(Vec3::new(0.0, CAMERA_Y_OFFSET, 0.0))
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
    ));
}

fn camera_follow_system(
    mut camera: Query<(&mut Transform, &GlobalTransform, &Camera), With<Camera3d>>,
    window: Query<&Window, With<PrimaryWindow>>,
    character: Query<&GlobalTransform, (With<LocalCharacterVisuals>, Without<Camera3d>)>,
) {
    if let (Ok(window), Ok((mut camera_transform, camera_global_transform, camera))) = (window.get_single(), camera.get_single_mut()) {
        if let Some(mouse_position) = window.cursor_position() {
            if let Ok(mouse_ray) = camera.viewport_to_world(&camera_global_transform, mouse_position) {
                if let Some(distance) = mouse_ray.intersect_plane(Vec3::ZERO, InfinitePlane3d { normal: Dir3::Y }) {
                    let mouse_world_position = mouse_ray.origin + (mouse_ray.direction * distance);

                    if let Ok(character_transform) = character.get_single() {
                        let char_pos = character_transform.translation();
                        
                        // First calculate the midpoint between character and mouse
                        let midpoint = (char_pos + mouse_world_position) * 0.5;
                        
                        // Then limit this midpoint's distance from character if needed
                        let to_midpoint = midpoint - char_pos;
                        let max_distance = CAMERA_Y_OFFSET;
                        
                        let limited_pos = if to_midpoint.length() > max_distance {
                            char_pos + to_midpoint.normalize() * max_distance
                        } else {
                            midpoint
                        };

                        camera_transform.translation = limited_pos + (Vec3::Y * CAMERA_Y_OFFSET);
                    }
                }
            }
        }
    }
}

/// Listens for new characters and tags them as local if they are ours.
fn tag_as_local_system(
    transport: Option<Res<NetcodeClientTransport>>,
    mut commands: Commands,
    characters: Query<(Entity, &Character)>,
) {
    let client_id = if let Some(transport) = transport {
        transport.client_id()
    } else {
        0
    };

    for (entity, character) in characters.iter() {
        if character.client_id == client_id {
            commands.entity(entity).insert(LocalCharacter);
        }
    }
}

/// Listens for new characters and spawns visuals for them.
fn spawn_visuals_system(
    transport: Option<Res<NetcodeClientTransport>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    characters: Query<(Entity, &Character), Added<Character>>,
) {
    for (entity, character) in characters.iter() {
        let client_id = if let Some(transport) = &transport {
            transport.client_id()
        } else {
            0
        };
    
        info!("Spawning visuals for character: {}", character.client_id);

        commands.entity(entity).insert(InheritedVisibility::default());

        if client_id == character.client_id {   
            commands.spawn((
                LocalCharacterVisuals,
                CharacterVisuals,
                MoveableVisuals {
                    simulation_entity: entity,
                },
                Mesh3d::from(meshes.add(Cuboid::default())),
                MeshMaterial3d::from(materials.add(Color::srgb(0.9, 0.1, 0.1))),
            ));
        } else {
            commands.spawn((
                CharacterVisuals,
                MoveableVisuals {
                    simulation_entity: entity,
                },
                Mesh3d::from(meshes.add(Cuboid::default())),
                MeshMaterial3d::from(materials.add(Color::srgb(0.9, 0.1, 0.1))),
            ));
        }
    }       
}
