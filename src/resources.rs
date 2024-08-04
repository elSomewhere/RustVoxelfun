// In a new file, e.g., resources.rs
use bevy::prelude::*;

#[derive(Resource)]
pub struct VoxelResources {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
}

#[derive(Component, Deref, DerefMut)]
pub struct InstanceMaterialData(pub Vec<Transform>);

