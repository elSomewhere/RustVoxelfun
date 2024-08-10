use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::view::NoFrustumCulling;
use bevy_flycam::prelude::*;

mod cube_mesh;
mod rendering;
mod types;

use crate::cube_mesh::create_cube_mesh;
use crate::rendering::{CustomMaterialPlugin, InstanceMaterialData};
use crate::types::InstanceData;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PlayerPlugin)
        .add_plugins(CustomMaterialPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {

    // Add a directional light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 100.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4),
            ..default()
        },
        ..default()
    });

    // Create instance data for 3 voxels
    let instances = vec![
        InstanceData {
            position: Vec3::new(0.0, 0.0, 0.0),
            scale: 1.0,
            color: [1.0, 0.0, 0.0, 1.0], // Red
            normal: [0.0, 1.0, 0.0],
            _padding: 0.0,
        },
        InstanceData {
            position: Vec3::new(2.0, 0.0, 0.0),
            scale: 1.0,
            color: [0.0, 1.0, 0.0, 1.0], // Green
            normal: [0.0, 1.0, 0.0],
            _padding: 0.0,
        },
        InstanceData {
            position: Vec3::new(1.0, 1.0, 0.0),
            scale: 1.0,
            color: [0.0, 0.0, 1.0, 1.0], // Blue
            normal: [0.0, 1.0, 0.0],
            _padding: 0.0,
        },
    ];

    // Spawn the instanced voxels
    commands.spawn((
        meshes.add(create_cube_mesh()),
        InstanceMaterialData(instances),
        SpatialBundle::INHERITED_IDENTITY,
        NoFrustumCulling,
    ));
}