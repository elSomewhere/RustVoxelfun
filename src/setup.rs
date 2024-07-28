use bevy::prelude::*;
use bevy::render::view::NoFrustumCulling;
use crate::{InstanceData, InstanceMaterialData};
use crate::cube_mesh::create_cube_mesh;

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {

    let custom_texture_handle: Handle<Image> = asset_server.load("/Users/estebanlanter/Documents/RustVoxelfun/src/array_texture.png");
    let cube_mesh_handle: Handle<Mesh> = meshes.add(create_cube_mesh());

    // commands.spawn((
    //     meshes.add(create_cube_mesh()),
    //     SpatialBundle::INHERITED_IDENTITY,
    //     InstanceMaterialData(vec![
    //         InstanceData {
    //             position: Vec3::new(-1.0, 0.0, 0.0),
    //             scale: 1.0,
    //             color: [1.0, 0.0, 0.0, 1.0],
    //         },
    //         InstanceData {
    //             position: Vec3::new(1.0, 0.0, 0.0),
    //             scale: 1.0,
    //             color: [0.0, 1.0, 0.0, 1.0],
    //         },
    //         InstanceData {
    //             position: Vec3::new(0.0, 1.0, 0.0),
    //             scale: 1.0,
    //             color: [0.0, 0.0, 1.0, 1.0],
    //         },
    //     ]),
    //     NoFrustumCulling,
    // ));

    // Remove or comment out the camera spawn code
    // commands.spawn(Camera3dBundle {
    //     transform: Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    //     ..default()
    // });
}
