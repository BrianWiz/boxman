use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::moveable_sim::{MoveableParams, MoveableSimulation};

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CharacterSpawnEvent>();
        app.add_event::<CharacterDespawnEvent>();
        app.add_systems(Update, (
            spawn_character_system,
            despawn_character_system,
        ));
    }
}

fn spawn_character_system(
    mut commands: Commands,
    mut character_spawn_events: EventReader<CharacterSpawnEvent>,
) {
    for event in character_spawn_events.read() {
        commands.spawn((
            MoveableSimulation {
                velocity: Vec3::ZERO,
                last_translation: event.position,
                last_rotation: Quat::IDENTITY,
                is_visually_correcting: false,
                grounded: false,
                params: MoveableParams {
                    gravity: 9.81,
                    collision_radius: 0.5,
                    collision_height: 1.0,
                    max_slope_angle: Some(44.0),
                },
            },
            Character {
                client_id: event.client_id,
            },
            Transform::from_translation(event.position),
        ));
    }
}

fn despawn_character_system(
    mut commands: Commands,
    mut character_despawn_events: EventReader<CharacterDespawnEvent>,
    visuals_query: Query<Entity, With<CharacterVisuals>>,
    character_query: Query<(Entity, &Character)>,
) {
    for event in character_despawn_events.read() {
        let all_visuals = visuals_query.iter().collect::<Vec<_>>();
        'outer: for (simulation_entity, character) in character_query.iter() {
            if character.client_id == event.client_id {
                'inner: for visuals_entity in all_visuals.iter() {
                    if *visuals_entity == simulation_entity {
                        commands.entity(*visuals_entity).despawn_recursive();
                        break 'inner;
                    }
                }
                commands.entity(simulation_entity).despawn_recursive();
                break 'outer;
            }
        }
    }
}

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
pub struct CharacterSpawnEvent {
    pub client_id: u64,
    pub position: Vec3,
    pub yaw: f32,
}

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
pub struct CharacterDespawnEvent {
    pub client_id: u64,
}

#[derive(Component)]
pub struct LocalCharacter;

#[derive(Component)]
pub struct LocalCharacterVisuals;


#[derive(Component)]
pub struct Character {
    pub client_id: u64,
}

#[derive(Component)]
pub struct CharacterVisuals;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInput {
    pub id: u32,
    pub snapshot_id: Option<u64>,
    pub yaw: f32,
    pub wish_dir: Vec2,
    pub wish_jump: bool,
    pub wish_fire: bool,
    pub active_weapon: u32,
    pub timestamp: f32,

    // Anything with a #[serde(skip)] attribute will not be sent over the network.

    #[serde(skip)]
    pub send_count: u32,

    #[serde(skip)]
    pub post_move_velocity: Vec3,

    #[serde(skip)]
    pub post_move_position: Vec3,

    #[serde(skip)]
    pub post_move_grounded: bool,
}

impl MoveableSimulation {
    fn apply_friction(velocity: Vec3, current_speed: f32, drag: f32, delta_seconds: f32) -> Vec3 {
        let mut new_speed;
        let mut drop = 0.0;
    
        drop += current_speed * drag * delta_seconds;
    
        new_speed = current_speed - drop;
        if new_speed < 0.0 {
            new_speed = 0.0;
        }
    
        if new_speed != 0.0 {
            new_speed /= current_speed;
        }
    
        velocity * new_speed
    }
    
    fn accelerate(
        wish_direction: Vec3,
        wish_speed: f32,
        current_speed: f32,
        accel: f32,
        delta_seconds: f32,
    ) -> Vec3 {
        let add_speed = wish_speed - current_speed;
    
        if add_speed <= 0.0 {
            return Vec3::ZERO;
        }
    
        let mut accel_speed = accel * delta_seconds * wish_speed;
        if accel_speed > add_speed {
            accel_speed = add_speed;
        }
    
        wish_direction * accel_speed
    }
}

pub fn alter_character_velocity(
    simulation: &mut MoveableSimulation,
    input: &PlayerInput,
    delta_secs: f32,
    speed: f32,
    acceleration: f32,
    friction: f32,
) {
    let rotation = Quat::from_rotation_y(input.yaw);
    let wish_dir = (rotation * Vec3::new(input.wish_dir.x, 0.0, input.wish_dir.y)).normalize_or_zero();
    
    simulation.velocity = MoveableSimulation::apply_friction(
        simulation.velocity,
        simulation.velocity.length(),
        friction * delta_secs,
        delta_secs
    );

    simulation.velocity += MoveableSimulation::accelerate(
        wish_dir,
        speed,
        simulation.velocity.dot(wish_dir),
        acceleration * delta_secs,
        delta_secs
    );
}
