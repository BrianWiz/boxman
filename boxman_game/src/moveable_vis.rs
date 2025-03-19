use bevy::prelude::*;
use bevy_renet::renet::RenetServer;
use boxman_shared::moveable_sim::{MoveableSimulation, MoveableVisuals};

use crate::config::MultiplayerConfig;

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
    mut visuals_query: Query<&mut Transform, With<MoveableVisuals>>,
    mut simulations_query: Query<(&Transform, &mut MoveableSimulation), Without<MoveableVisuals>>,
) {
    // for (simulation_transform, mut simulation) in simulations_query.iter_mut() {
    //     if let Some(visuals) = simulation.visuals() {
    //         if let Ok(mut visual_transform) = visuals_query.get_mut(visuals) {
    //             let last_translation = simulation.last_translation;
    //             // Are we smooth correcting from a server correction?
    //             if simulation.is_visually_correcting {
    //                 let target = last_translation.lerp(
    //                     simulation_transform.translation,
    //                     fixed_time.overstep_fraction()
    //                 );
                    
    //                 let current_distance = visual_transform.translation.distance(target);
    //                 let framerate_adjusted_factor = cfg.moveable_correction_position_factor.powf(time.delta_secs() * 60.0);
    //                 let lerp_amount = (framerate_adjusted_factor * current_distance).min(current_distance);
                    
    //                 visual_transform.translation = visual_transform.translation.lerp(
    //                     target,
    //                     lerp_amount / current_distance
    //                 );

    //                 if current_distance < 0.01 {
    //                     simulation.is_visually_correcting = false;
    //                 }
    //             }
    //             // Or are we just moving normally?
    //             else {
    //                 // Lerp between last known position and current position based on fixed timestep progress
    //                 visual_transform.translation = last_translation.lerp(
    //                     simulation_transform.translation,
    //                     fixed_time.overstep_fraction()
    //                 );
    //             }

    //             if server.is_some() {
    //                 // interpolate rotation on the server, because the server is receving inputs at a fixed tick.
    //                 // And if the listen server is rendering higher than that, we need to keep it smooth.
    //                 visual_transform.rotation = simulation.last_rotation.slerp(
    //                     simulation_transform.rotation,
    //                     fixed_time.overstep_fraction()
    //                 );
    //             } else {
    //                 // This is fine because we update the simulation's rotation every Update frame already
    //                 visual_transform.rotation = simulation_transform.rotation;
    //             }
    //         }
    //     }
    // }
}

