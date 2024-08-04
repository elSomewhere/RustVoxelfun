use bevy::prelude::*;
use bevy_flycam::prelude::*;
use crate::chunk::{apply_chunk_updates, prepare_chunk_updates, remove_marked_chunks};
use crate::cube_mesh::create_cube_mesh;
use crate::interaction::handle_mouse_input;
use crate::resources::VoxelResources;
use crate::terrain::TerrainState;
use crate::world::{mark_chunks_for_update, update_marked_chunks};

mod cube_mesh;
mod terrain;
mod chunk;
mod interaction;
mod world;
mod resources;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PlayerPlugin)
        .insert_resource(TerrainState::default())
        .add_systems(Startup, setup_lighting)
        .add_systems(Startup, setup_voxel_resources)
        .add_systems(Update, mark_chunks_for_update)
        .add_systems(Update, update_marked_chunks)
        .add_systems(Update, prepare_chunk_updates)
        .add_systems(Update, apply_chunk_updates)
        .add_systems(Update, remove_marked_chunks)
        .add_systems(Update, handle_mouse_input)
        .run();
}

fn setup_lighting(mut commands: Commands) {
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

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.2,
    });
}


pub fn setup_voxel_resources(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_mesh = meshes.add(create_cube_mesh());
    let voxel_material = materials.add(StandardMaterial {
        base_color: Color::rgb(0.8, 0.7, 0.6),
        ..default()
    });

    commands.insert_resource(VoxelResources {
        mesh: cube_mesh,
        material: voxel_material,
    });
}