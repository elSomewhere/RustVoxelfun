use std::collections::HashMap;
use std::slice::Windows;
use bevy::prelude::*;
use bevy::input::mouse::MouseButtonInput;
use bevy::window::PrimaryWindow;
use bevy_flycam::FlyCam;
use crate::chunk::{Chunk, VOXEL_SIZE, CHUNK_SIZE};

use bevy::window::Window;
use crate::terrain::TerrainState;

use bevy::ecs::system::ParamSet;

pub fn handle_mouse_input(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<FlyCam>>,
    mut terrain_state: ResMut<TerrainState>,
    mut chunk_query: ParamSet<(Query<&Chunk>, Query<&mut Chunk>)>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let (camera, camera_transform) = camera_query.single();
        let window = window_query.single();

        if let Some(cursor_position) = window.cursor_position() {
            if let Some(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
                let max_distance = 10.0;
                let chunks = chunk_query.p0();
                if let Some((chunk_pos, voxel_pos, _normal)) = raycast(ray.origin, ray.direction.into(), max_distance, &terrain_state, &chunks) {
                    info!("Hit voxel at chunk {:?}, local pos {:?}", chunk_pos, voxel_pos);
                    let mut chunks_mut = chunk_query.p1();
                    remove_voxel(&mut terrain_state, &mut chunks_mut, chunk_pos, voxel_pos);
                } else {
                    info!("No voxel hit");
                }
            }
        }
    }
}

fn raycast(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    terrain_state: &TerrainState,
    chunks: &Query<&Chunk>,
) -> Option<(IVec3, IVec3, Vec3)> {
    let step = 0.01; // Smaller step for more precise detection
    let mut current_pos = origin;

    for _ in 0..((max_distance / step) as i32) {
        current_pos += direction.normalize() * step;

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

                let neighbor_entities = get_chunk_neighbors(&terrain_state.chunks, chunk_pos);
                let neighbors: [Option<&Chunk>; 6] = neighbor_entities.map(|entity| {
                    entity.and_then(|e| chunks.get(e).ok())
                });

                if chunk.is_voxel_solid_raycast(local_pos.x, local_pos.y, local_pos.z, &neighbors) {
                    // Calculate the normal of the hit face
                    let normal = calculate_hit_normal(current_pos, direction);
                    return Some((chunk_pos, local_pos, normal));
                }
            }
        }
    }

    None
}

fn calculate_hit_normal(hit_pos: Vec3, ray_direction: Vec3) -> Vec3 {
    let epsilon = 0.01;
    let x = hit_pos.x.fract();
    let y = hit_pos.y.fract();
    let z = hit_pos.z.fract();

    if x < epsilon && ray_direction.x < 0.0 {
        Vec3::new(-1.0, 0.0, 0.0)
    } else if x > 1.0 - epsilon && ray_direction.x > 0.0 {
        Vec3::new(1.0, 0.0, 0.0)
    } else if y < epsilon && ray_direction.y < 0.0 {
        Vec3::new(0.0, -1.0, 0.0)
    } else if y > 1.0 - epsilon && ray_direction.y > 0.0 {
        Vec3::new(0.0, 1.0, 0.0)
    } else if z < epsilon && ray_direction.z < 0.0 {
        Vec3::new(0.0, 0.0, -1.0)
    } else if z > 1.0 - epsilon && ray_direction.z > 0.0 {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        // Fallback: use the opposite of the ray direction
        -ray_direction.normalize()
    }
}

fn remove_voxel(
    terrain_state: &mut TerrainState,
    chunks: &mut Query<&mut Chunk>,
    chunk_pos: IVec3,
    voxel_pos: IVec3,
) {
    if let Some(&chunk_entity) = terrain_state.chunks.get(&chunk_pos) {
        if let Ok(mut chunk) = chunks.get_mut(chunk_entity) {
            chunk.remove_voxel(voxel_pos.x, voxel_pos.y, voxel_pos.z);

            // Mark the chunk for update
            terrain_state.chunks_to_update.insert(chunk_pos);

            // Mark neighboring chunks for update if the voxel is on the edge
            if voxel_pos.x == 0 { terrain_state.chunks_to_update.insert(chunk_pos + IVec3::new(-1, 0, 0)); }
            if voxel_pos.x == CHUNK_SIZE - 1 { terrain_state.chunks_to_update.insert(chunk_pos + IVec3::new(1, 0, 0)); }
            if voxel_pos.y == 0 { terrain_state.chunks_to_update.insert(chunk_pos + IVec3::new(0, -1, 0)); }
            if voxel_pos.y == CHUNK_SIZE - 1 { terrain_state.chunks_to_update.insert(chunk_pos + IVec3::new(0, 1, 0)); }
            if voxel_pos.z == 0 { terrain_state.chunks_to_update.insert(chunk_pos + IVec3::new(0, 0, -1)); }
            if voxel_pos.z == CHUNK_SIZE - 1 { terrain_state.chunks_to_update.insert(chunk_pos + IVec3::new(0, 0, 1)); }
        }
    }
}

fn get_chunk_neighbors(chunks: &HashMap<IVec3, Entity>, chunk_pos: IVec3) -> [Option<Entity>; 6] {
    let neighbor_positions = [
        IVec3::new(-1, 0, 0),
        IVec3::new(1, 0, 0),
        IVec3::new(0, -1, 0),
        IVec3::new(0, 1, 0),
        IVec3::new(0, 0, -1),
        IVec3::new(0, 0, 1),
    ];

    neighbor_positions.map(|offset| chunks.get(&(chunk_pos + offset)).copied())
}