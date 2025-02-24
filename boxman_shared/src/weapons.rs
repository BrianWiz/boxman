use std::time::Duration;

use bevy::{prelude::*, utils::hashbrown::HashMap};
use bevy_config_stack::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{player::LocalPlayerControllerSimulation, types::{MaterialKind, MeshConfig, MeshKind}, utils::Server};

pub type WeaponKey = u32;

#[derive(Resource, Default)]
pub struct WeaponAssetCache {
    pub projectile_meshes: HashMap<WeaponKey, Handle<Mesh>>,
    pub projectile_materials: HashMap<WeaponKey, Handle<StandardMaterial>>,
}

#[derive(Asset, TypePath, Debug, Resource, Serialize, Deserialize, Default)]
pub struct WeaponsListConfig {
    pub weapons: Vec<WeaponConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WeaponConfig {
    pub key: WeaponKey,
    pub name: String,
    pub fire_rate_ms: u32,
    pub projectile_config: ProjectileConfig,
}

impl Default for WeaponConfig {
    fn default() -> Self {
        Self {
            key: 0,
            name: "Default Weapon".to_string(),
            fire_rate_ms: 100,
            projectile_config: ProjectileConfig {
                speed: 10.0,
                gravity_multiplier: 1.0,
                lifetime: 10.0,
                collision_radius: 1.0,
                mesh_config: MeshConfig {
                    mesh_kind: MeshKind::Sphere { radius: 1.0 },
                    material_kind: MaterialKind::Standard { color: Color::WHITE },
                    offset: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                },
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectileConfig {
    pub speed: f32,
    pub gravity_multiplier: f32,
    pub lifetime: f32,
    pub collision_radius: f32,
    pub mesh_config: MeshConfig,
}

pub struct Weapon {
    pub key: WeaponKey,
    pub fire_timer: Timer,
    pub wish_fire: bool,
}

#[derive(Component)]
pub struct Inventory {
    pub weapons: Vec<Weapon>,
    pub active_weapon: WeaponKey,
}

pub struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeaponAssetCache>();
        app.add_systems(Update, cache_weapons_assets_system);
        app.add_systems(FixedUpdate, update_weapons_system.run_if(resource_exists::<WeaponsListConfig>));
    }
}

fn cache_weapons_assets_system(
    mut weapon_asset_cache: ResMut<WeaponAssetCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    weapons_list_config: Option<Res<WeaponsListConfig>>,
    mut config_asset_loaded_event: EventReader<ConfigAssetLoadedEvent<WeaponsListConfig>>,
) {
    for _ in config_asset_loaded_event.read() {
        if let Some(ref config) = weapons_list_config {
            for weapon in config.weapons.iter() {
                let mesh_handle = match weapon.projectile_config.mesh_config.mesh_kind {
                    MeshKind::Sphere { radius } => {
                        let mesh = Sphere::new(radius);
                        meshes.add(mesh)
                    }
                    MeshKind::Box { width, height, depth } => {
                        let mesh = Cuboid::new(width, height, depth);
                        meshes.add(mesh)
                    }
                    MeshKind::Capsule { radius, height } => {
                        let mesh = Capsule3d::new(radius, height);
                        meshes.add(mesh)
                    }
                    _ => {
                        unimplemented!()
                    }
                };

                let material_handle = match weapon.projectile_config.mesh_config.material_kind {
                    MaterialKind::Standard { color } => {
                        let material = Color::from(color);
                        materials.add(material)
                    }
                };

                weapon_asset_cache.projectile_meshes.insert(weapon.key, mesh_handle);
                weapon_asset_cache.projectile_materials.insert(weapon.key, material_handle);
            }
        }
    }
}

fn update_weapons_system(
    cfg: Res<WeaponsListConfig>,
    mut local_inventories: Query<&mut Inventory, With<LocalPlayerControllerSimulation>>,
    mut inventories: Query<&mut Inventory, Without<LocalPlayerControllerSimulation>>,
    fixed_time: Res<Time<Fixed>>,
    server: Option<Res<Server>>,
) {
    if server.is_some() {
        for mut inventory in inventories.iter_mut() {
            if inventory.weapons.len() <= inventory.active_weapon as usize {
                panic!("Active weapon index out of bounds");
            }

            let active_weapon_key = inventory.active_weapon as usize;
            let weapon = &mut inventory.weapons[active_weapon_key];

            weapon.fire_timer.tick(fixed_time.delta());

            if weapon.wish_fire && weapon.fire_timer.finished() {
                weapon.fire_timer.reset();

                info!("Firing weapon: {}", weapon.key);
            }
        }
    }

    for mut inventory in local_inventories.iter_mut() {
        if inventory.weapons.len() <= inventory.active_weapon as usize {
            panic!("Active weapon index out of bounds");
        }

        let active_weapon_key = inventory.active_weapon as usize;
        let weapon = &mut inventory.weapons[active_weapon_key];
        weapon.fire_timer.tick(fixed_time.delta());
        if weapon.wish_fire && weapon.fire_timer.finished() {
            if let Some(weapon_config) = cfg.weapons.get(active_weapon_key) {
                weapon.fire_timer.set_duration(Duration::from_millis(weapon_config.fire_rate_ms as u64));
            }
            weapon.fire_timer.reset();

            info!("Firing weapon: {}", weapon.key);
        }
    }
}
