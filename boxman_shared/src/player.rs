use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::moveable_sim::{MoveableParams, MoveableSimulation};

pub const PLAYER_CONTROLLER_SPEED: f32 = 8.0;
pub const PLAYER_CONTROLLER_JUMP_IMPULSE: f32 = 3.8;
pub const PLAYER_CONTROLLER_GROUND_FRICTION: f32 = 14.0;
pub const PLAYER_CONTROLLER_AIR_FRICTION: f32 = 2.0;
pub const PLAYER_CONTROLLER_GROUND_ACCEL: f32 = 10.0;
pub const PLAYER_CONTROLLER_AIR_ACCEL: f32 = 2.0;
pub const PLAYER_CONTROLLER_AIR_SPEED_MULTIPLIER: f32 = 0.7;

#[derive(Component)]
pub struct LocalPlayerControllerSimulation;

#[derive(Component)]
pub struct LocalPlayerControllerVisuals;

#[derive(Component)]
pub struct PlayerControllerSimulation {
    pub client_id: u64,
}

#[derive(Component)]
pub struct PlayerControllerVisuals;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInput {
    pub id: u32,
    pub snapshot_id: Option<u64>,
    pub yaw: f32,
    pub wish_dir: Vec2,
    pub wish_jump: bool,
    pub client_timestamp: f32,

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

pub fn spawn_player_controller(
    commands: &mut Commands,
    position: Vec3,
    client_id: u64,
    local: bool,
    meshes: Option<&mut Assets<Mesh>>,
    materials: Option<&mut Assets<StandardMaterial>>,
) {
    let entities = MoveableSimulation::spawn(
        commands,
        if let Some(meshes) = meshes {
            Some(meshes.add(Cuboid::new(1.0, 1.0, 1.0)))
        } else {
            None
        },
        if let Some(materials) = materials {
            Some(materials.add(Color::srgb(0.5, 0.1, 0.1)))
        } else {
            None
        },
        position,
        MoveableParams {
            gravity: 9.81,
            collision_radius: 0.5,
            collision_height: 1.0,
            max_slope_angle: Some(44.0),
        },
    );

    // Simulation
    commands.entity(entities.simulation)
        .insert(PlayerControllerSimulation {
            client_id: client_id,
        });

    if local {
        commands.entity(entities.simulation)
            .insert(LocalPlayerControllerSimulation);
    }

    // Visuals
    if let Some(visuals) = entities.visuals {
        commands.entity(visuals)
            .insert(PlayerControllerVisuals);

        if local {
            commands.entity(visuals)
                .insert(LocalPlayerControllerVisuals);
        }
    }
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

pub fn alter_player_controller_velocity(
    simulation: &mut MoveableSimulation,
    input: &PlayerInput,
    delta_secs: f32,
    speed: f32,
    jump_impulse: f32,
    ground_accel: f32,
    air_accel: f32,
    ground_friction: f32,
    air_friction: f32,
) {
    let rotation = Quat::from_rotation_y(input.yaw);
    let wish_dir = (rotation * Vec3::new(input.wish_dir.x, 0.0, input.wish_dir.y)).normalize_or_zero();
    
    simulation.velocity = MoveableSimulation::apply_friction(
        simulation.velocity,
        simulation.velocity.length(),
        if simulation.grounded { ground_friction } else { air_friction },
        delta_secs
    );

    simulation.velocity += MoveableSimulation::accelerate(
        wish_dir,
        speed,
        simulation.velocity.dot(wish_dir),
        if simulation.grounded { ground_accel } else { air_accel },
        delta_secs
    );

    if input.wish_jump && simulation.grounded {
        simulation.velocity.y += jump_impulse;
    }
}

pub fn despawn_player_controller(
    client_id: u64,
    commands: &mut Commands,
    player_controllers: &Query<(Entity, &MoveableSimulation, &PlayerControllerSimulation)>,
) {
    for (entity, simulation, controller) in player_controllers.iter() {
        if controller.client_id == client_id {
            commands.entity(entity).despawn_recursive();
            if let Some(visuals) = simulation.visuals() {
                commands.entity(visuals).despawn_recursive();
            }
        }
    }
}
