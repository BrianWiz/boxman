use std::time::Duration;

use bevy::{prelude::*, utils::HashMap};
use bevy_renet::renet::ServerEvent;
use boxman_shared::{
    moveable_sim::MoveableSimulation, 
    player::{alter_character_velocity, despawn_character, spawn_character, CharacterSimulation, PlayerInput, PLAYER_CONTROLLER_AIR_ACCEL, PLAYER_CONTROLLER_AIR_FRICTION, PLAYER_CONTROLLER_GROUND_ACCEL, PLAYER_CONTROLLER_GROUND_FRICTION, PLAYER_CONTROLLER_JUMP_IMPULSE, PLAYER_CONTROLLER_SPEED}, weapons::Inventory
};
use rand::Rng;

#[derive(Resource)]
pub struct BotIdTracker {
    pub next_bot_id: u64,
}

impl Default for BotIdTracker {
    fn default() -> Self {
        Self { next_bot_id: 9999 }
    }
}

#[derive(Component)]
pub struct Bot {
    /// The client id of the bot
    pub id: u64,
    /// How often it tries to dodge
    pub dodge_timer: Timer,
    /// Its active attack target
    pub attack_target: Option<Entity>,
    /// The chance to attack and jump
    pub attack_and_jump_chance: f32,
}

pub struct BotsPlugin;

impl Plugin for BotsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BotIdTracker::default());
        app.add_systems(FixedPreUpdate, (
            target_seeker_system,
            bot_movement_system
        ));
    }
}

pub fn spawn_bot(
    bot_id_tracker: &mut BotIdTracker,
    commands: &mut Commands, 
    meshes: &mut Assets<Mesh>, 
    materials: &mut Assets<StandardMaterial>,
    initial_position: Vec3,
) {
    let client_id = bot_id_tracker.next_bot_id;
    bot_id_tracker.next_bot_id += 1;
    let entities = spawn_character(
        commands,
        initial_position,
        client_id,
        false, // Not local
        Some(meshes),
        Some(materials),
    );
    commands.entity(entities.simulation).insert(Bot {
        id: client_id,
        dodge_timer: Timer::new(Duration::from_secs(1), TimerMode::Once),
        attack_target: None,
        attack_and_jump_chance: 0.5,
    });
}

fn target_seeker_system(
    mut bot_query: Query<(Entity, &mut Bot)>,
    player_controllers: Query<&CharacterSimulation>,
) {
    for (entity, mut bot) in bot_query.iter_mut() {

        if bot.attack_target.is_some() {
            continue;
        }

        // @todo-brian: We should do a line trace to find the closest visible player controller
        for controller in player_controllers.iter() {
            if controller.client_id == bot.id {
                continue;
            }

            bot.attack_target = Some(entity);
            break;
        }
    }
}

/// System to handle bot movement and actions
fn bot_movement_system(
    time: Res<Time<Fixed>>,
    mut bot_query: Query<(Entity, &mut Bot, &mut Inventory, &mut MoveableSimulation), With<CharacterSimulation>>,
    mut transforms: Query<(Entity, &mut Transform), With<CharacterSimulation>>,
) {
    let mut rng = rand::rng();
    
    let transform_positions: Vec<(Entity, Vec3)> = transforms
        .iter()
        .map(|(entity, transform)| (entity, transform.translation))
        .collect();
    
    for (bot_entity, mut bot, mut inventory, mut simulation) in bot_query.iter_mut() {
        if let Ok((_, mut bot_transform)) = transforms.get_mut(bot_entity) {
            let active_weapon_key = inventory.active_weapon as usize;
            let active_weapon = &mut inventory.weapons[active_weapon_key];
            
            let input = if let Some(target_entity) = bot.attack_target {

                let target_pos = transform_positions.iter()
                    .find(|(entity, _)| *entity == target_entity)
                    .map(|(_, pos)| *pos);
                
                if let Some(target_position) = target_pos {
                    let direction = target_position - bot_transform.translation;
                    let horizontal_dir = Vec2::new(direction.x, direction.z);
                    
                    if horizontal_dir.length() > 5.0 {
                        let target_yaw = horizontal_dir.y.atan2(horizontal_dir.x);
                        bot_transform.rotation = Quat::from_rotation_y(target_yaw);
                        let wish_dir = Vec2::new(target_yaw.cos(), target_yaw.sin());

                        PlayerInput {
                            id: rng.random(),
                            snapshot_id: None,
                            yaw: target_yaw,
                            wish_dir,
                            wish_jump: rng.random_bool(bot.attack_and_jump_chance as f64),
                            wish_fire: true,
                            active_weapon: active_weapon_key as u32,
                            timestamp: time.elapsed_secs(),
                            send_count: 0,
                            post_move_velocity: Vec3::ZERO,
                            post_move_position: Vec3::ZERO,
                            post_move_grounded: false,
                        }
                    } else {
                        let yaw = horizontal_dir.y.atan2(horizontal_dir.x);
                        
                        PlayerInput {
                            id: rng.random(),
                            snapshot_id: None,
                            yaw,
                            wish_dir: Vec2::ZERO,
                            wish_jump: false,
                            wish_fire: true,
                            active_weapon: active_weapon_key as u32,
                            timestamp: time.elapsed_secs(),
                            send_count: 0,
                            post_move_velocity: Vec3::ZERO,
                            post_move_position: Vec3::ZERO,
                            post_move_grounded: false,
                        }
                    }
                } else {
                    // Target exists but we couldn't find its position
                    // Clear the target and use random movement
                    bot.attack_target = None;
                    create_random_input(&mut rng, &bot_transform, active_weapon_key, time.elapsed_secs())
                }
            } else {
                // No target, use random movement
                create_random_input(&mut rng, &bot_transform, active_weapon_key, time.elapsed_secs())
            };
            
            alter_character_velocity(
                &mut simulation,
                &input,
                time.delta_secs(),
                PLAYER_CONTROLLER_SPEED,
                PLAYER_CONTROLLER_JUMP_IMPULSE,
                PLAYER_CONTROLLER_GROUND_ACCEL,
                PLAYER_CONTROLLER_AIR_ACCEL,
                PLAYER_CONTROLLER_GROUND_FRICTION,
                PLAYER_CONTROLLER_AIR_FRICTION,
            );
            
            active_weapon.wish_fire = input.wish_fire;
        }
    }
}

/// Helper function to create a random movement input
fn create_random_input(
    rng: &mut impl Rng,
    transform: &Transform,
    active_weapon_key: usize,
    timestamp: f32,
) -> PlayerInput {
    let yaw = rng.random_range(0.0..std::f32::consts::TAU);
    
    PlayerInput {
        id: rng.random(),
        snapshot_id: None,
        yaw,
        wish_dir: Vec2::new(rng.random_range(-1.0..1.0), rng.random_range(-1.0..1.0)).normalize_or_zero(),
        wish_jump: false,
        wish_fire: false,
        active_weapon: active_weapon_key as u32,
        timestamp,
        send_count: 0,
        post_move_velocity: Vec3::ZERO,
        post_move_position: Vec3::ZERO,
        post_move_grounded: false,
    }
}
