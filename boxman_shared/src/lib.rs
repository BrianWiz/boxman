pub mod moveable_sim;
pub mod protocol;
pub mod character;
pub mod snapshot;
pub mod types;
pub mod utils;

pub mod prelude {
    pub use super::*;
    pub use character::*;
    pub use moveable_sim::*;
    pub use protocol::*;
    pub use snapshot::*;
    pub use types::*;
    pub use utils::*;
}

use bevy::prelude::*;
use character::*;
use moveable_sim::MoveableSimulationPlugin;

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CharacterPlugin);
        app.add_plugins(MoveableSimulationPlugin);
    }
}
