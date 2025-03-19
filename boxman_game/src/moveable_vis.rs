use bevy::prelude::*;
use bevy_renet::renet::RenetServer;
use boxman_shared::moveable_sim::{MoveableSimulation, MoveableVisuals};
use boxman_shared::data::MultiplayerConfig;

#[derive(Component)]
pub struct VisualErrorOffset {
    pub position_error: Vec3,
    pub orientation_error: Quat,
}

pub struct MoveableVisualsPlugin;

impl Plugin for MoveableVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, 
            visuals_interpolation_system.run_if(resource_exists::<MultiplayerConfig>)
        );
    }
}

fn visuals_interpolation_system(
    time: Res<Time>,
    fixed_time: Res<Time<Fixed>>,
    server: Option<Res<RenetServer>>,
    cfg: Res<MultiplayerConfig>,
    mut visuals_query: Query<(&mut Transform, &MoveableVisuals)>,
    mut simulations_query: Query<(&Transform, &mut MoveableSimulation), Without<MoveableVisuals>>,
) {
    for (mut visuals_transform, visuals) in visuals_query.iter_mut() {
        if let Ok((simulation_transform, mut simulation)) = simulations_query.get_mut(visuals.simulation_entity) {
            let last_translation = simulation.last_translation;
            
            // Are we smooth correcting from a server correction?
            if simulation.is_visually_correcting {
                let target = last_translation.lerp(
                    simulation_transform.translation,
                    fixed_time.overstep_fraction()
                );
                let current_distance = visuals_transform.translation.distance(target);
                
                let blend = ((current_distance - cfg.visual_smooth_min_threshold) / cfg.visual_smooth_range).clamp(0.0, 1.0);
                let smooth_factor = cfg.visual_smooth_factor_min * (1.0 - blend) + cfg.visual_smooth_factor_max * blend;
                
                // Calculate movement this frame
                let max_move_distance = current_distance * smooth_factor * time.delta_secs() * cfg.visual_smooth_speed_multiplier;
                let move_dir = (target - visuals_transform.translation).normalize_or_zero();
                
                if current_distance > cfg.visual_snap_threshold {
                    // Move towards target, but limit the movement to prevent overshooting
                    visuals_transform.translation += move_dir * max_move_distance;
                } else {
                    visuals_transform.translation = target;
                    simulation.is_visually_correcting = false;
                }
            }
            // Or are we just moving normally?
            else {
                // Lerp between last known position and current position based on fixed timestep progress
                visuals_transform.translation = last_translation.lerp(
                    simulation_transform.translation,
                    fixed_time.overstep_fraction()
                );
            }

            if server.is_some() {
                // Interpolate rotation on the server since it receives inputs at a fixed tick
                visuals_transform.rotation = simulation.last_rotation.slerp(
                    simulation_transform.rotation,
                    fixed_time.overstep_fraction()
                );
            } else {
                // This is fine because we update the simulation's rotation every Update frame already
                visuals_transform.rotation = simulation_transform.rotation;
            }
        }
    }
}

