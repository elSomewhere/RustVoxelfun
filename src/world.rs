use bevy::prelude::*;
use bevy_flycam::FlyCam;
use crate::terrain::TerrainState;
use crate::chunk::{Chunk, CHUNK_SIZE, RENDER_DISTANCE, TERRAIN_HEIGHT, VOXEL_SIZE};
use crate::cube_mesh::create_cube_mesh;
use crate::resources::VoxelResources;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TerrainState::default())
            .add_systems(Update, mark_chunks_for_update)
            .add_systems(Update, update_marked_chunks);
    }
}

pub fn mark_chunks_for_update(
    mut terrain_state: ResMut<TerrainState>,
    query: Query<&Transform, With<FlyCam>>,
) {
    let player_position = query.single().translation;
    let new_player_chunk = IVec3::new(
        (player_position.x / (CHUNK_SIZE as f32 * VOXEL_SIZE)).floor() as i32,
        0,
        (player_position.z / (CHUNK_SIZE as f32 * VOXEL_SIZE)).floor() as i32,
    );

    // Calculate player's position within the chunk
    let player_local_position = Vec3::new(
        player_position.x % (CHUNK_SIZE as f32 * VOXEL_SIZE),
        player_position.y,
        player_position.z % (CHUNK_SIZE as f32 * VOXEL_SIZE),
    );

    // Print player information every time, even if the chunk hasn't changed
    // info!("Player global position: {:?}", player_position);
    // info!("Player chunk: {:?}", new_player_chunk);
    // info!("Player position within chunk: {:?}", player_local_position);

    // Only update if the player has moved to a new chunk
    if new_player_chunk != terrain_state.player_chunk {
        terrain_state.player_chunk = new_player_chunk;

        // Clear the previous update and remove sets
        terrain_state.chunks_to_update.clear();
        terrain_state.chunks_to_remove.clear();

        // Mark chunks for spawning or updating, and identify chunks to remove, in a single pass
        let mut positions_to_remove = Vec::new();

        for x in -RENDER_DISTANCE-1..=RENDER_DISTANCE+1 {
            for z in -RENDER_DISTANCE-1..=RENDER_DISTANCE+1 {
                let current_chunk_position = new_player_chunk + IVec3::new(x, 0, z);

                if x.abs() <= RENDER_DISTANCE && z.abs() <= RENDER_DISTANCE {
                    // This chunk is within render distance
                    if !terrain_state.chunks.contains_key(&current_chunk_position) {
                        info!("Marking new chunk for creation: {:?}", current_chunk_position);
                    } else {
                        info!("Marking existing chunk for update: {:?}", current_chunk_position);
                    }
                    terrain_state.chunks_to_update.insert(current_chunk_position);
                } else if terrain_state.chunks.contains_key(&current_chunk_position) {
                    // This chunk is outside render distance and exists, so mark for removal
                    info!("Marking chunk for removal: {:?}", current_chunk_position);
                    positions_to_remove.push(current_chunk_position);
                }
            }
        }

        // Add the positions to remove to the terrain state
        terrain_state.chunks_to_remove.extend(positions_to_remove);

        // Print debug information
        info!("Player chunk: {:?}", terrain_state.player_chunk);
        info!("Chunks to update: {:?}", terrain_state.chunks_to_update);
        info!("Chunks to remove: {:?}", terrain_state.chunks_to_remove);
    }
}

pub fn update_marked_chunks(
    mut commands: Commands,
    mut terrain_state: ResMut<TerrainState>,
    voxel_resources: Res<VoxelResources>,
    mut chunks: Query<&mut Chunk>,
) {
    let chunks_to_update = terrain_state.chunks_to_update.clone();

    for &chunk_pos in &chunks_to_update {
        if !terrain_state.chunks.contains_key(&chunk_pos) {
            info!("Creating new chunk at position: {:?}", chunk_pos);
            let chunk = Chunk::new(
                chunk_pos,
                CHUNK_SIZE as u32,
                TERRAIN_HEIGHT,
                CHUNK_SIZE as u32,
            );
            let chunk_entity = chunk.create_voxel_entities(&mut commands, voxel_resources.mesh.clone(), voxel_resources.material.clone());
            commands.entity(chunk_entity).insert(chunk);
            terrain_state.chunks.insert(chunk_pos, chunk_entity);
        } else {
            info!("Updating chunk at position: {:?}", chunk_pos);
            if let Some(&chunk_entity) = terrain_state.chunks.get(&chunk_pos) {
                if let Ok(mut chunk) = chunks.get_mut(chunk_entity) {
                    chunk.update_voxel_entities(&mut commands, chunk_entity);
                }
            }
        }
    }

    terrain_state.chunks_to_update.clear();
}