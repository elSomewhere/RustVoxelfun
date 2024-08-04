use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_flycam::FlyCam;
use bevy_xpbd_3d::parry::query::Ray;
use crate::chunk::{Chunk, VOXEL_SIZE, CHUNK_SIZE};
use crate::terrain::TerrainState;

pub fn handle_mouse_input(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<FlyCam>>,
    mut terrain_state: ResMut<TerrainState>,
    mut chunk_query: Query<&mut Chunk>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let (camera, camera_transform) = camera_query.single();
        let window = window_query.single();

        let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);

        if let Some(ray) = camera.viewport_to_world(camera_transform, center) {
            let max_distance = 10.0;
            if let Some((chunk_pos, voxel_pos)) = raycast(&ray, max_distance, &terrain_state, &chunk_query) {
                info!("Hit voxel at chunk {:?}, local pos {:?}", chunk_pos, voxel_pos);
                if let Some(&chunk_entity) = terrain_state.chunks.get(&chunk_pos) {
                    if let Ok(mut chunk) = chunk_query.get_mut(chunk_entity) {
                        chunk.remove_voxel(voxel_pos.x, voxel_pos.y, voxel_pos.z);
                        chunk.dirty = true;
                        terrain_state.chunks_to_update.insert(chunk_pos);
                    }
                }
            } else {
                info!("No voxel hit");
            }
        }
    }
}

fn raycast(
    ray: &Ray3d,
    max_distance: f32,
    terrain_state: &TerrainState,
    chunks: &Query<&mut Chunk>,
) -> Option<(IVec3, IVec3)> {
    let step = 0.1;
    let mut current_pos = ray.origin;

    for _ in 0..((max_distance / step) as i32) {
        current_pos += ray.direction.normalize() * step;

        let chunk_pos = IVec3::new(
            (current_pos.x / (CHUNK_SIZE as f32 * VOXEL_SIZE)).floor() as i32,
            0,
            (current_pos.z / (CHUNK_SIZE as f32 * VOXEL_SIZE)).floor() as i32,
        );

        if let Some(&chunk_entity) = terrain_state.chunks.get(&chunk_pos) {
            if let Ok(chunk) = chunks.get(chunk_entity) {
                let local_pos = IVec3::new(
                    (current_pos.x.rem_euclid(CHUNK_SIZE as f32 * VOXEL_SIZE) / VOXEL_SIZE).floor() as i32,
                    current_pos.y.floor() as i32,
                    (current_pos.z.rem_euclid(CHUNK_SIZE as f32 * VOXEL_SIZE) / VOXEL_SIZE).floor() as i32,
                );

                if chunk.is_voxel_solid(local_pos.x, local_pos.y, local_pos.z) {
                    return Some((chunk_pos, local_pos));
                }
            }
        }
    }

    None
}