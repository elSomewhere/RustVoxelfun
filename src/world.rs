use bevy::prelude::*;
use bevy::render::mesh::PrimitiveTopology;
use bevy_flycam::FlyCam;
use crate::terrain::TerrainState;
use crate::chunk::{Chunk, CHUNK_SIZE, create_chunk_mesh, RENDER_DISTANCE, TERRAIN_HEIGHT, VOXEL_SIZE};
use crate::DefaultMaterial;

pub fn update_marked_chunks(
    mut commands: Commands,
    mut terrain_state: ResMut<TerrainState>,
    mut meshes: ResMut<Assets<Mesh>>,
    default_material: Res<DefaultMaterial>,
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
            let mesh = create_chunk_mesh(&chunk);
            let chunk_entity = commands.spawn((
                chunk,
                PbrBundle {
                    mesh: meshes.add(mesh),
                    material: default_material.0.clone(),
                    transform: Transform::from_translation(Vec3::new(
                        chunk_pos.x as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE,
                        0.0,
                        chunk_pos.z as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE,
                    )),
                    ..default()
                },
            )).id();
            terrain_state.chunks.insert(chunk_pos, chunk_entity);
        } else {
            info!("Chunk at position is already there: {:?}", chunk_pos);
        }
    }

    terrain_state.chunks_to_update.clear();
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