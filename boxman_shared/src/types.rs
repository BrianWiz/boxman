use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum MeshKind {
    Box {
        width: f32,
        height: f32,
        depth: f32,
    },
    Sphere {
        radius: f32,
    },
    Capsule {
        radius: f32,
        height: f32,
    },
    Model {
        path: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MaterialKind {
    Standard {
        color: Color,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeshConfig {
    pub mesh_kind: MeshKind,
    pub material_kind: MaterialKind,
    pub offset: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}