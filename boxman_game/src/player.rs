use bevy::{input::mouse::MouseMotion, prelude::*};
use boxman_shared::{moveable_sim::MoveableSimulation, player::{alter_player_controller_velocity, LocalPlayerControllerSimulation, LocalPlayerControllerVisuals, PlayerInput, PLAYER_CONTROLLER_AIR_ACCEL, PLAYER_CONTROLLER_AIR_FRICTION, PLAYER_CONTROLLER_GROUND_ACCEL, PLAYER_CONTROLLER_GROUND_FRICTION, PLAYER_CONTROLLER_JUMP_IMPULSE, PLAYER_CONTROLLER_SPEED}};

use crate::net::snapshot::LastProcessedSnapshotId;
use crate::config::ControlsConfig;

const CAMERA_HEIGHT_OFFSET: f32 = 0.25;

#[derive(Resource)]
pub struct PlayerControllerInputHistory {
    pub next_input_id: u32,
    pub inputs: Vec<PlayerInput>,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera_system);
        app.add_systems(PreUpdate, look_controls_system.run_if(resource_exists::<ControlsConfig>));
        app.add_systems(FixedPreUpdate, 
            (
                input_capture_system
                    .run_if(resource_exists::<ControlsConfig>),
                alter_velocity_system
            )
            .chain()
        );
        app.add_systems(FixedPostUpdate, post_move_system);
        app.add_systems(PostUpdate, camera_follow_system);
        app.insert_resource(PlayerControllerInputHistory {
            next_input_id: 0,
            inputs: Vec::new(),
        });
    }
}

fn input_capture_system(
    time: Res<Time<Fixed>>,
    mut input_history: ResMut<PlayerControllerInputHistory>,
    snapshot_id: Option<ResMut<LastProcessedSnapshotId>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    player_controller: Query<(Entity, &Transform, &MoveableSimulation), With<LocalPlayerControllerSimulation>>,
) {
    // We always send an input to the server regardless of whether we have a player controller or not.
    // So always create an input history entry.
    let player_controller = player_controller.get_single();
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
                direction += Vec2::NEG_Y;
            }
            if keyboard_input.pressed(KeyCode::KeyS) {
                direction += Vec2::Y;
            }
            if keyboard_input.pressed(KeyCode::KeyA) {
                direction += Vec2::NEG_X;
            }
            if keyboard_input.pressed(KeyCode::KeyD) {
                direction += Vec2::X;
            }
            direction.normalize_or_zero()
        },
        wish_jump: {
            if let Ok((_, _, moveable_simulation)) = player_controller {
                keyboard_input.pressed(KeyCode::Space) && moveable_simulation.grounded
            } else {
                false
            }
        },
        send_count: 0,
        client_timestamp: time.elapsed_secs(),
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

fn look_controls_system(
    settings: Res<ControlsConfig>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
    mut player_controller: Query<&mut Transform, (With<LocalPlayerControllerSimulation>, Without<Camera3d>)>,
) {
    if let (Ok(mut camera_transform), Ok(mut player_transform)) = (camera.get_single_mut(), player_controller.get_single_mut()) {
        for motion in mouse_motion.read() {
            // Rotate around global Y axis for left/right look
            player_transform.rotate_axis(Dir3::Y, -motion.delta.x * settings.mouse_sensitivity);

            // Rotate around local X axis for up/down look
            let pitch = -motion.delta.y * settings.mouse_sensitivity;
            let pitch_rot = camera_transform.rotation;
            camera_transform.rotate_local_x(pitch);

            // Clamp the up/down rotation
            let up_dot = (camera_transform.rotation * Vec3::Z).y;
            if up_dot.abs() > 0.99 {
                camera_transform.rotation = pitch_rot;
            }
        }
    }
}

fn alter_velocity_system(
    fixed_time: Res<Time<Fixed>>,
    mut player_controller: Query<&mut MoveableSimulation, (With<LocalPlayerControllerSimulation>, Without<Camera3d>)>,
    mut player_inputs: ResMut<PlayerControllerInputHistory>,
) {
    if let Ok(mut player_controller) = player_controller.get_single_mut() {
        if let Some(input) = player_inputs.inputs.last_mut() {
            alter_player_controller_velocity(
                &mut player_controller, 
                input, 
                fixed_time.delta_secs(), 
                PLAYER_CONTROLLER_SPEED, 
                PLAYER_CONTROLLER_JUMP_IMPULSE,
                PLAYER_CONTROLLER_GROUND_ACCEL,
                PLAYER_CONTROLLER_AIR_ACCEL,
                PLAYER_CONTROLLER_GROUND_FRICTION, 
                PLAYER_CONTROLLER_AIR_FRICTION, 
            );
        }
    }
}

fn post_move_system(
    mut player_inputs: ResMut<PlayerControllerInputHistory>,
    player_controller: Query<(&MoveableSimulation, &Transform), With<LocalPlayerControllerSimulation>>,
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
        Transform::default()
            .with_translation(Vec3::new(0.0, 1.0, 5.0))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn camera_follow_system(
    mut camera: Query<&mut Transform, With<Camera3d>>,
    player: Query<&Transform, (With<LocalPlayerControllerVisuals>, Without<Camera3d>)>,
) {
    if let (Ok(mut camera), Ok(player)) = (camera.get_single_mut(), player.get_single()) {
        camera.translation = player.translation + (Vec3::Y * CAMERA_HEIGHT_OFFSET);
        
        let (_, camera_pitch, _) = camera.rotation.to_euler(EulerRot::YXZ);
        let (player_yaw, _, _) = player.rotation.to_euler(EulerRot::YXZ);
        camera.rotation = Quat::from_euler(EulerRot::YXZ, player_yaw, camera_pitch, 0.0);
    }
}
