use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Asset, TypePath, Debug, Resource, Serialize, Deserialize)]
pub struct ControlsConfig {
    pub mouse_sensitivity: f32,
    pub controls: Controls,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Controls {
    pub move_forward: ControlsInput,
    pub move_backward: ControlsInput,
    pub move_left: ControlsInput,
    pub move_right: ControlsInput,
    pub jump: ControlsInput,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ControlsInput {
    Keyboard(KeyCode),
    Mouse(MouseButton),
}

impl Default for ControlsConfig {
    fn default() -> Self {
        Self { 
            mouse_sensitivity: 0.001,
            controls: Controls {
                move_forward: ControlsInput::Keyboard(KeyCode::KeyW),
                move_backward: ControlsInput::Keyboard(KeyCode::KeyS),
                move_left: ControlsInput::Keyboard(KeyCode::KeyA),
                move_right: ControlsInput::Keyboard(KeyCode::KeyD),
                jump: ControlsInput::Keyboard(KeyCode::Space),
            },
        }
    }
}

#[derive(Asset, TypePath, Debug, Resource, Serialize, Deserialize)]
pub struct MultiplayerConfig {
    pub minimum_correction_duration: f32,
    pub maximum_correction_duration: f32,
    pub correction_distance_scale: f32,
    pub server_rotation_interpolation_speed: f32,
}

impl Default for MultiplayerConfig {
    fn default() -> Self {
        Self {
            minimum_correction_duration: 0.001,
            maximum_correction_duration: 0.2,
            correction_distance_scale: 0.1,
            server_rotation_interpolation_speed: 20.0,
        }
    }
}
