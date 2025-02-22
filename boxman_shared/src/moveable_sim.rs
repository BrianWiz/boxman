use bevy::prelude::*;
use avian3d::prelude::*;

const GROUND_MARGIN: f32 = 0.001;

pub type MaxSlopeAngleDegrees = f32;

pub struct MoveableEntities {
    pub simulation: Entity,
    pub visuals: Option<Entity>,
}

pub struct MoveableParams {
    pub gravity: f32,
    pub collision_radius: f32,
    pub collision_height: f32,

    /// The maximum slope angle that the moveable can stand on. 
    /// If set, a grounded check will be performed.
    pub max_slope_angle: Option<MaxSlopeAngleDegrees>,
}

#[derive(Component)]
pub struct MoveableVisuals;

pub struct MoveableCorrectionState {
    pub from: Vec3,
    pub correction_timer: Timer,
}

#[derive(Component)]
pub struct MoveableSimulation {
    visuals: Option<Entity>,
    pub velocity: Vec3,
    pub last_translation: Vec3,
    pub correction_state: Option<MoveableCorrectionState>,
    pub last_rotation: Quat,
    pub params: MoveableParams,
    pub grounded: bool,
}

impl MoveableSimulation {
    pub fn spawn(
        commands: &mut Commands,
        mesh: Option<Handle<Mesh>>,
        material: Option<Handle<StandardMaterial>>,
        spawn_position: Vec3,
        params: MoveableParams,
    ) -> MoveableEntities {
        
        let visuals = if let (Some(mesh), Some(material)) = (mesh, material) {
            Some(commands.spawn((
                Mesh3d::from(mesh),
                MeshMaterial3d::from(material),
                Transform::from_translation(spawn_position),
                MoveableVisuals,
            )).id())
        } else {
            None
        };

        let simulation = commands.spawn((
            // Collider::capsule(params.collision_radius, params.collision_height),
            MoveableSimulation {
                visuals,
                velocity: Vec3::ZERO,
                last_translation: spawn_position,
                last_rotation: Quat::IDENTITY,
                params,
                correction_state: None,
                grounded: false,
            },
            Transform::from_translation(spawn_position),
        )).id();

        MoveableEntities {
            simulation,
            visuals,
        }
    }

    pub fn last_rotation(&self) -> Quat {
        self.last_rotation
    }

    pub fn visuals(&self) -> Option<Entity> {
        self.visuals
    }
}

pub struct MoveableSimulationPlugin;

impl Plugin for MoveableSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (
            simulation_move_system,
        ));
    }
}

fn simulation_move_system(
    fixed_time: Res<Time<Fixed>>,
    spatial_query: SpatialQuery,
    mut simulations: Query<(Entity, &mut MoveableSimulation, &mut Transform)>,
) {
    for (entity, mut simulation, mut transform) in simulations.iter_mut() {
        move_simulation(
            &fixed_time,
            &spatial_query,
            &mut simulation,
            &mut transform,
            entity,
        );
    }
}

pub fn move_simulation(
    fixed_time: &Time<Fixed>,
    spatial_query: &SpatialQuery,
    simulation: &mut MoveableSimulation,
    transform: &mut Transform,
    entity: Entity,
) {
    const EPSILON: f32 = 0.001;

    let collider = Collider::cylinder(simulation.params.collision_radius, simulation.params.collision_height);

    simulation.last_translation = transform.translation;
    simulation.last_rotation = transform.rotation;

    simulation.velocity.y -= simulation.params.gravity * fixed_time.delta_secs();

    let mut velocity = simulation.velocity;
    let mut remaining_motion = velocity * fixed_time.delta_secs();

    let mut grounded_this_frame = false;

    for _ in 0..4 {

        if let Some(hit) = spatial_query.cast_shape(
            &collider,
            transform.translation,
            Quat::default(),
            Dir3::new(remaining_motion.normalize_or_zero()).unwrap_or(Dir3::X),
            &ShapeCastConfig::from_max_distance(remaining_motion.length()),
            &SpatialQueryFilter::default().with_excluded_entities([entity]),
        ) {
            // Move to just before the collision point
            transform.translation += remaining_motion.normalize_or_zero() * hit.distance;

            // Prevents sticking
            transform.translation += hit.normal1 * EPSILON;

            // Deflect velocity along the surface
            velocity -= hit.normal1 * velocity.dot(hit.normal1);
            remaining_motion -= hit.normal1 * remaining_motion.dot(hit.normal1);
            
            if !grounded_this_frame {
                if let Some(max_slope_angle_degrees) = simulation.params.max_slope_angle {
                    let slope_angle = hit.normal1.y.acos();
                    if slope_angle < max_slope_angle_degrees.to_radians() {
                        // Snaps to the ground
                        transform.translation.y = hit.point1.y + (simulation.params.collision_height * 0.5) + GROUND_MARGIN;
                        grounded_this_frame = true;
                    }
                }
            }
        } else {
            // No collision, move the full distance
            transform.translation += remaining_motion;
            break;
        }
    }

    simulation.velocity = velocity;
    simulation.grounded = grounded_this_frame;
}
