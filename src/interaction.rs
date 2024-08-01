use std::slice::Windows;
use bevy::prelude::*;
use bevy::input::mouse::MouseButtonInput;
use bevy::window::PrimaryWindow;
use bevy_flycam::FlyCam;
use crate::chunk::{Chunk, VOXEL_SIZE, TerrainState, CHUNK_SIZE};
use crate::terrain::Terrain;

use bevy::window::Window;

pub(crate) fn handle_mouse_input(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<FlyCam>>,
    terrain_state: Res<TerrainState>,
    chunks: Query<&Chunk>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let (camera, camera_transform) = camera_query.single();
        let window = window_query.single();

        if let Some(cursor_position) = window.cursor_position() {
            info!("Cursor position: {:?}", cursor_position);

            if let Some(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
                info!("Ray origin: {:?}, direction: {:?}", ray.origin, ray.direction);

                info!("TerrainState has {} chunks", terrain_state.chunks.len());

                if let Some((chunk_pos, voxel_pos)) = raycast(&ray, &chunks, &terrain_state) {
                    info!("Hit voxel at chunk {:?}, local pos {:?}", chunk_pos, voxel_pos);
                    // Handle voxel removal or modification here
                } else {
                    info!("No voxel hit");
                }
            } else {
                info!("Failed to create ray from cursor position");
            }
        } else {
            info!("No cursor position available");
        }
    }
}

fn raycast(
    ray: &Ray3d,
    chunks: &Query<&Chunk>,
    terrain_state: &TerrainState,
) -> Option<(IVec3, IVec3)> {
    let max_distance = 100.0;
    let step = 0.1;

    info!("Starting raycast from {:?} in direction {:?}", ray.origin, ray.direction);
    info!("TerrainState has {} chunks", terrain_state.chunks.len());

    let mut current_pos = ray.origin;
    for i in 0..((max_distance / step) as i32) {
        let chunk_pos = IVec3::new(
            (current_pos.x / (CHUNK_SIZE as f32)).floor() as i32,
            (current_pos.y / (CHUNK_SIZE as f32)).floor() as i32,
            (current_pos.z / (CHUNK_SIZE as f32)).floor() as i32,
        );
        if let Some(&chunk_entity) = terrain_state.chunks.get(&chunk_pos) {
            info!("Found chunk at position {:?}", chunk_pos);
            if let Ok(chunk) = chunks.get(chunk_entity) {
                let local_pos = IVec3::new(
                    (current_pos.x.rem_euclid(CHUNK_SIZE as f32)) as i32,
                    (current_pos.y.rem_euclid(CHUNK_SIZE as f32)) as i32,
                    (current_pos.z.rem_euclid(CHUNK_SIZE as f32)) as i32,
                );

                info!("Checking voxel at chunk {:?}, local pos {:?}", chunk_pos, local_pos);

                // Create an array of neighboring chunks
                let neighbors: [Option<&Chunk>; 6] = [
                    terrain_state.chunks.get(&(chunk_pos + IVec3::X)).and_then(|&e| chunks.get(e).ok()),
                    terrain_state.chunks.get(&(chunk_pos - IVec3::X)).and_then(|&e| chunks.get(e).ok()),
                    terrain_state.chunks.get(&(chunk_pos + IVec3::Y)).and_then(|&e| chunks.get(e).ok()),
                    terrain_state.chunks.get(&(chunk_pos - IVec3::Y)).and_then(|&e| chunks.get(e).ok()),
                    terrain_state.chunks.get(&(chunk_pos + IVec3::Z)).and_then(|&e| chunks.get(e).ok()),
                    terrain_state.chunks.get(&(chunk_pos - IVec3::Z)).and_then(|&e| chunks.get(e).ok()),
                ];

                if chunk.is_voxel_solid(local_pos.x, local_pos.y, local_pos.z, &neighbors) {
                    info!("Found solid voxel at chunk {:?}, local pos {:?}", chunk_pos, local_pos);
                    return Some((chunk_pos, local_pos));
                }
            }
        } else {
            if i % 10 == 0 {
                info!("No chunk found at position {:?}", chunk_pos);
            }
        }

        current_pos += ray.direction * step;
    }

    info!("No voxel hit within max distance");
    None
}

fn remove_voxel(terrain_state: &mut TerrainState, chunks: &mut Query<&mut Chunk>, chunk_pos: IVec3, voxel_pos: IVec3) {
    info!("Attempting to remove voxel at chunk {:?}, position {:?}", chunk_pos, voxel_pos);

    if let Some(&chunk_entity) = terrain_state.chunks.get(&chunk_pos) {
        if let Ok(mut chunk) = chunks.get_mut(chunk_entity) {
            chunk.remove_voxel(voxel_pos.x, voxel_pos.y, voxel_pos.z);
            info!("Voxel removed");
            terrain_state.chunks_to_update.insert(chunk_pos);

            // Mark neighboring chunks for update if the voxel is on the edge
            if voxel_pos.x == 0 { terrain_state.chunks_to_update.insert(chunk_pos + IVec3::new(-1, 0, 0)); }
            if voxel_pos.x == 15 { terrain_state.chunks_to_update.insert(chunk_pos + IVec3::new(1, 0, 0)); }
            if voxel_pos.z == 0 { terrain_state.chunks_to_update.insert(chunk_pos + IVec3::new(0, 0, -1)); }
            if voxel_pos.z == 15 { terrain_state.chunks_to_update.insert(chunk_pos + IVec3::new(0, 0, 1)); }
        } else {
            error!("Failed to get mutable reference to chunk");
        }
    } else {
        error!("Chunk not found in terrain state");
    }
}