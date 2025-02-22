use bevy::prelude::*;
use boxman_shared::moveable_sim::{MoveableSimulation, MoveableVisuals};

pub struct MoveableVisualsPlugin;

impl Plugin for MoveableVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, 
            visuals_interpolation_system
        );
    }
}

fn visuals_interpolation_system(
    time: Res<Time>,
    fixed_time: Res<Time<Fixed>>,
    mut visuals_query: Query<&mut Transform, With<MoveableVisuals>>,
    mut simulations_query: Query<(&Transform, &mut MoveableSimulation), Without<MoveableVisuals>>,
) {
    for (simulation_transform, mut simulation) in simulations_query.iter_mut() {
        if let Some(visuals) = simulation.visuals() {
            if let Ok(mut visual_transform) = visuals_query.get_mut(visuals) {
                let last_translation = simulation.last_translation;
                // Are we smooth correcting from a server correction?
                if let Some(ref mut correction) = simulation.correction_state {
                    correction.correction_timer.tick(time.delta());
                    let progress = correction.correction_timer.elapsed_secs() / correction.correction_timer.duration().as_secs_f32();
                    let correction_target = correction.from.lerp(
                        simulation_transform.translation,
                        progress
                    );
                    visual_transform.translation = correction_target;
                    if correction.correction_timer.finished() {
                        simulation.correction_state = None;
                    }
                }
                // Or are we just moving normally?
                else {
                    // Lerp between last known position and current position based on fixed timestep progress
                    visual_transform.translation = last_translation.lerp(
                        simulation_transform.translation,
                        fixed_time.overstep_fraction()
                    );
                }

                // This is fine because we update the simulation's rotation every Update frame already
                visual_transform.rotation = simulation_transform.rotation;
            }
        }
    }
}

