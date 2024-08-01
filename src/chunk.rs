use bevy::prelude::*;
use bevy::render::view::NoFrustumCulling;
use noise::{NoiseFn, Perlin};
use crate::terrain::Terrain;

use crate::cube_mesh::create_cube_mesh;
use crate::{InstanceData, InstanceMaterialData};
use std::collections::{HashMap, HashSet};
use bevy_flycam::FlyCam;

pub const CHUNK_SIZE: i32 = 16;
pub const RENDER_DISTANCE: i32 = 3;
pub const TERRAIN_HEIGHT: u32 = 64;
pub const VOXEL_SIZE: f32 = 1.0;

#[derive(Component)]
pub struct Chunk {
    pub position: IVec3,
    width: u32,
    height: u32,
    depth: u32,
    voxels: Vec<bool>,
}

impl Chunk {
    pub fn new(position: IVec3, width: u32, height: u32, depth: u32) -> Self {
        let mut voxels = vec![false; (width * height * depth) as usize];
        let perlin = Perlin::new(0);

        for x in 0..width {
            for z in 0..depth {
                let world_x = position.x * CHUNK_SIZE as i32 + x as i32;
                let world_z = position.z * CHUNK_SIZE as i32 + z as i32;
                let height_value = ((perlin.get([world_x as f64 / 50.0, world_z as f64 / 50.0]) + 1.0) * (TERRAIN_HEIGHT as f64 / 2.0)) as u32;
                for y in 0..height_value.min(height) {
                    voxels[(x + y * width + z * width * height) as usize] = true;
                }
            }
        }

        Self {
            position,
            width,
            height,
            depth,
            voxels,
        }
    }

    fn calculate_normal(&self, x: u32, y: u32, z: u32, neighbors: &[Option<&Chunk>; 6]) -> [f32; 3] {
        let mut normal = [0.0, 0.0, 0.0];
        let directions = [(-1, 0, 0), (1, 0, 0), (0, -1, 0), (0, 1, 0), (0, 0, -1), (0, 0, 1)];

        for (i, (dx, dy, dz)) in directions.iter().enumerate() {
            let (nx, ny, nz) = (x as i32 + dx, y as i32 + dy, z as i32 + dz);

            if !self.is_voxel_solid(nx, ny, nz, neighbors) {
                normal[i / 2] += *dx as f32 + *dy as f32 + *dz as f32;
            }
        }

        normal
    }

    pub fn create_instance_data(&self, neighbors: &[Option<&Chunk>; 6]) -> Vec<InstanceData> {
        let mut instances = Vec::new();

        for x in 0..self.width {
            for y in 0..self.height {
                for z in 0..self.depth {
                    if self.voxels[(x + y * self.width + z * self.width * self.height) as usize] {
                        let world_x = self.position.x * CHUNK_SIZE as i32 + x as i32;
                        let world_y = self.position.y * CHUNK_SIZE as i32 + y as i32;
                        let world_z = self.position.z * CHUNK_SIZE as i32 + z as i32;

                        if self.is_voxel_visible(x as i32, y as i32, z as i32, neighbors) {
                            let normal = self.calculate_normal(x, y, z, neighbors);
                            let height_ratio = y as f32 / TERRAIN_HEIGHT as f32;
                            let color = [
                                0.2 + height_ratio * 0.3,
                                0.4 + height_ratio * 0.2,
                                0.1 + height_ratio * 0.1,
                                1.0,
                            ];
                            instances.push(InstanceData {
                                position: Vec3::new(world_x as f32 * VOXEL_SIZE, world_y as f32 * VOXEL_SIZE, world_z as f32 * VOXEL_SIZE),
                                scale: VOXEL_SIZE,
                                color,
                                normal,
                                _padding: 0.0,
                            });
                        }
                    }
                }
            }
        }

        instances
    }

    fn is_voxel_visible(&self, x: i32, y: i32, z: i32, neighbors: &[Option<&Chunk>; 6]) -> bool {
        let directions = [(-1, 0, 0), (1, 0, 0), (0, -1, 0), (0, 1, 0), (0, 0, -1), (0, 0, 1)];

        for (dx, dy, dz) in directions.iter() {
            let (nx, ny, nz) = (x + dx, y + dy, z + dz);
            if !self.is_voxel_solid(nx, ny, nz, neighbors) {
                return true;
            }
        }

        false
    }

    fn is_voxel_solid(&self, x: i32, y: i32, z: i32, neighbors: &[Option<&Chunk>; 6]) -> bool {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 && z >= 0 && z < self.depth as i32 {
            return self.voxels[(x + y * self.width as i32 + z * self.width as i32 * self.height as i32) as usize];
        }

        let (chunk_x, chunk_y, chunk_z) = (
            if x < 0 { -1 } else if x >= CHUNK_SIZE { 1 } else { 0 },
            if y < 0 { -1 } else if y >= self.height as i32 { 1 } else { 0 },
            if z < 0 { -1 } else if z >= CHUNK_SIZE { 1 } else { 0 },
        );

        let neighbor_index = match (chunk_x, chunk_y, chunk_z) {
            (-1, 0, 0) => 0,
            (1, 0, 0) => 1,
            (0, -1, 0) => 2,
            (0, 1, 0) => 3,
            (0, 0, -1) => 4,
            (0, 0, 1) => 5,
            _ => return false, // Corner or edge case, treat as air
        };

        if let Some(neighbor) = &neighbors[neighbor_index] {
            let (nx, ny, nz) = (
                (x + CHUNK_SIZE) % CHUNK_SIZE,
                y.rem_euclid(self.height as i32),
                (z + CHUNK_SIZE) % CHUNK_SIZE,
            );
            neighbor.voxels[(nx + ny * CHUNK_SIZE + nz * CHUNK_SIZE * neighbor.height as i32) as usize]
        } else {
            false // If there's no neighbor chunk, treat it as air
        }
    }
}


pub fn update_terrain(
    mut commands: Commands,
    mut terrain_state: ResMut<Terrain>,
    query: Query<&Transform, With<FlyCam>>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_query: Query<(Entity, &Chunk)>,
    mut instance_query: Query<&mut InstanceMaterialData>,
) {
    let player_position = query.single().translation;
    let chunk_position = IVec3::new(
        (player_position.x / (CHUNK_SIZE as f32 * VOXEL_SIZE)).floor() as i32,
        0,
        (player_position.z / (CHUNK_SIZE as f32 * VOXEL_SIZE)).floor() as i32,
    );

    // Spawn new chunks
    for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
        for z in -RENDER_DISTANCE..=RENDER_DISTANCE {
            let current_chunk_position = chunk_position + IVec3::new(x, 0, z);
            if !terrain_state.chunks.contains_key(&current_chunk_position) {
                let chunk = Chunk::new(
                    current_chunk_position,
                    CHUNK_SIZE as u32,
                    TERRAIN_HEIGHT,
                    CHUNK_SIZE as u32,
                );
                let chunk_entity = commands.spawn((
                    chunk,
                    meshes.add(create_cube_mesh()),
                    SpatialBundle::INHERITED_IDENTITY,
                    InstanceMaterialData(Vec::new()),
                    NoFrustumCulling,
                )).id();
                terrain_state.chunks.insert(current_chunk_position, chunk_entity);
            }
        }
    }

    // Remove chunks that are too far away
    terrain_state.chunks.retain(|&pos, &mut entity| {
        if (pos - chunk_position).abs().max_element() > RENDER_DISTANCE {
            commands.entity(entity).despawn();
            false
        } else {
            true
        }
    });

    // Update instance data for all chunks
    for (&chunk_pos, &entity) in terrain_state.chunks.iter() {
        if let Ok((_, chunk)) = chunk_query.get(entity) {
            let mut neighbors = [None; 6];
            let neighbor_positions = [
                IVec3::new(-1, 0, 0),
                IVec3::new(1, 0, 0),
                IVec3::new(0, -1, 0),
                IVec3::new(0, 1, 0),
                IVec3::new(0, 0, -1),
                IVec3::new(0, 0, 1),
            ];

            for (i, offset) in neighbor_positions.iter().enumerate() {
                if let Some(&neighbor_entity) = terrain_state.chunks.get(&(chunk_pos + *offset)) {
                    if let Ok((_, neighbor_chunk)) = chunk_query.get(neighbor_entity) {
                        neighbors[i] = Some(neighbor_chunk);
                    }
                }
            }

            if let Ok(mut instance_material_data) = instance_query.get_mut(entity) {
                let new_instance_data = chunk.create_instance_data(&neighbors);
                instance_material_data.0 = new_instance_data;
            }
        }
    }
}


#[derive(Resource, Default)]
pub struct TerrainState {
    pub chunks: HashMap<IVec3, Entity>,
    pub chunks_to_update: HashSet<IVec3>,
    pub chunks_to_remove: HashSet<IVec3>,
}
#[derive(Component)]
pub struct ChunkNeedsUpdate;

pub fn mark_chunks_for_update(
    mut terrain_state: ResMut<TerrainState>,
    query: Query<&Transform, With<FlyCam>>,
) {
    let player_position = query.single().translation;
    let chunk_position = IVec3::new(
        (player_position.x / (CHUNK_SIZE as f32 * VOXEL_SIZE)).floor() as i32,
        0,
        (player_position.z / (CHUNK_SIZE as f32 * VOXEL_SIZE)).floor() as i32,
    );

    // Clear the previous update and remove sets
    terrain_state.chunks_to_update.clear();
    terrain_state.chunks_to_remove.clear();

    // Mark chunks for spawning or updating
    for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
        for z in -RENDER_DISTANCE..=RENDER_DISTANCE {
            let current_chunk_position = chunk_position + IVec3::new(x, 0, z);
            terrain_state.chunks_to_update.insert(current_chunk_position);
        }
    }

    // Collect positions to remove
    let positions_to_remove: Vec<IVec3> = terrain_state.chunks.keys()
        .filter(|&&pos| (pos - chunk_position).abs().max_element() > RENDER_DISTANCE)
        .cloned()
        .collect();

    // Mark chunks for removal
    for pos in positions_to_remove {
        terrain_state.chunks_to_remove.insert(pos);
    }
}

pub fn update_marked_chunks(
    mut commands: Commands,
    mut terrain_state: ResMut<TerrainState>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_query: Query<(Entity, &Chunk)>,
    mut instance_query: Query<&mut InstanceMaterialData>,
) {
    let chunks_to_update = terrain_state.chunks_to_update.clone();
    let chunks_to_remove = terrain_state.chunks_to_remove.clone();

    // Handle chunk updates and creation
    for &chunk_pos in &chunks_to_update {
        if let Some(&entity) = terrain_state.chunks.get(&chunk_pos) {
            // Update existing chunk
            if let Ok((_, chunk)) = chunk_query.get(entity) {
                let neighbor_entities = get_chunk_neighbors(&terrain_state.chunks, chunk_pos);
                let neighbors: [Option<&Chunk>; 6] = neighbor_entities.map(|entity| {
                    entity.and_then(|e| chunk_query.get(e).ok().map(|(_, chunk)| chunk))
                });

                if let Ok(mut instance_material_data) = instance_query.get_mut(entity) {
                    let new_instance_data = chunk.create_instance_data(&neighbors);
                    instance_material_data.0 = new_instance_data;
                }
            }
        } else {
            // Spawn new chunk
            let chunk = Chunk::new(
                chunk_pos,
                CHUNK_SIZE as u32,
                TERRAIN_HEIGHT,
                CHUNK_SIZE as u32,
            );
            let chunk_entity = commands.spawn((
                chunk,
                meshes.add(create_cube_mesh()),
                SpatialBundle::INHERITED_IDENTITY,
                InstanceMaterialData(Vec::new()),
                NoFrustumCulling,
            )).id();
            terrain_state.chunks.insert(chunk_pos, chunk_entity);
        }
    }

    // Handle chunk removal
    for &chunk_pos in &chunks_to_remove {
        if let Some(entity) = terrain_state.chunks.remove(&chunk_pos) {
            commands.entity(entity).despawn();
        }
    }

    // Clear the update and remove sets
    terrain_state.chunks_to_update.clear();
    terrain_state.chunks_to_remove.clear();
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