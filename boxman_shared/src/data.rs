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
    /// Minimum distance (in world units) that triggers smoothing interpolation.
    /// When correction distance is below this value, only the min smoothing factor is applied.
    /// This creates a "stable zone" for small network jitter.
    pub visual_smooth_min_threshold: f32,   

    /// Width of the interpolation blend zone (in world units).
    /// Defines how smoothing transitions from min to max factor:
    /// - Start: min_threshold
    /// - End: min_threshold + range
    /// Larger values create more gradual transitions between smooth/fast corrections.
    pub visual_smooth_range: f32,           

    /// Smoothing coefficient for small corrections [0.0 to 1.0].
    /// Applied when correction distance <= min_threshold.
    /// Acts as a velocity dampening factor:
    /// - 1.0 = No correction (infinite smoothing)
    /// - 0.0 = Instant correction (no smoothing)
    pub visual_smooth_factor_min: f32,      

    /// Smoothing coefficient for large corrections [0.0 to 1.0].
    /// Applied when correction distance >= (min_threshold + range).
    /// Lower values create faster corrections for large network errors.
    /// Should be < factor_min to ensure large errors are corrected quickly.
    pub visual_smooth_factor_max: f32,      

    /// Base movement speed multiplier for corrections.
    /// Final correction velocity = distance * smooth_factor * delta_time * multiplier
    /// Higher values reduce correction time but may cause overshooting.
    /// Tune this based on typical network update frequency.
    pub visual_smooth_speed_multiplier: f32, 

    /// Distance threshold for position snapping (in world units).
    /// When correction distance falls below this value:
    /// - Position instantly snaps to target
    /// - Correction state is cleared
    /// Should be small enough to be visually unnoticeable.
    pub visual_snap_threshold: f32,         
}

impl Default for MultiplayerConfig {
    fn default() -> Self {
        Self {
            visual_smooth_min_threshold: 0.1,
            visual_smooth_range: 0.4,
            visual_smooth_factor_min: 0.98,
            visual_smooth_factor_max: 0.90,
            visual_smooth_speed_multiplier: 15.0,
            visual_snap_threshold: 0.005,
        }
    }
}

#[derive(Asset, TypePath, Debug, Resource, Serialize, Deserialize)]
pub struct CharacterConfig {
    pub speed: f32,
    pub acceleration: f32,
    pub friction: f32,
}

impl Default for CharacterConfig {
    fn default() -> Self {
        Self {
            speed: 100.0,
            acceleration: 10.0,
            friction: 4.0,
        }
    }
}
