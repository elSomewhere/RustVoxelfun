use bevy::prelude::*;
use bevy::render::view::NoFrustumCulling;
use noise::{NoiseFn, Perlin};

use crate::cube_mesh::create_cube_mesh;
use crate::{InstanceData, InstanceMaterialData};
use std::collections::{HashMap, HashSet};
use bevy_flycam::FlyCam;
use crate::terrain::TerrainState;

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
    pub dirty: bool,
    cached_instance_data: Option<Vec<InstanceData>>,
}


#[derive(Component)]
pub struct PreparedInstanceData(pub Vec<InstanceData>);

pub fn prepare_chunk_updates(
    terrain_state: Res<TerrainState>,
    chunk_query: Query<&Chunk>,
    mut commands: Commands,
) {
    for &chunk_pos in &terrain_state.chunks_to_update {
        if let Some(&entity) = terrain_state.chunks.get(&chunk_pos) {
            if let Ok(chunk) = chunk_query.get(entity) {
                if chunk.dirty {
                    let neighbor_entities = get_chunk_neighbors(&terrain_state.chunks, chunk_pos);
                    let neighbors: [Option<&Chunk>; 6] = neighbor_entities.map(|entity| {
                        entity.and_then(|e| chunk_query.get(e).ok())
                    });

                    let new_instance_data = chunk.generate_instance_data(&neighbors);
                    commands.entity(entity).insert(PreparedInstanceData(new_instance_data));
                }
            }
        }
    }
}

pub fn apply_chunk_updates(
    mut chunk_query: Query<(Entity, &mut Chunk, &mut InstanceMaterialData, Option<&PreparedInstanceData>)>,
    mut commands: Commands,
) {
    for (entity, mut chunk, mut instance_material_data, prepared_data) in chunk_query.iter_mut() {
        if let Some(prepared_data) = prepared_data {
            instance_material_data.0 = prepared_data.0.clone();
            chunk.dirty = false;
            commands.entity(entity).remove::<PreparedInstanceData>();
        }
    }
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
            dirty: true, // New chunk is initially dirty
            cached_instance_data: None,
        }
    }

    pub fn set_voxel(&mut self, x: i32, y: i32, z: i32, is_solid: bool) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 && z >= 0 && z < self.depth as i32 {
            let index = (x + y * self.width as i32 + z * self.width as i32 * self.height as i32) as usize;
            if self.voxels[index] != is_solid {
                self.voxels[index] = is_solid;
                self.dirty = true;
            }
        }
    }

    pub fn remove_voxel(&mut self, x: i32, y: i32, z: i32) {
        self.set_voxel(x, y, z, false);
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

    pub fn create_instance_data(&mut self, neighbors: &[Option<&Chunk>; 6]) -> Vec<InstanceData> {
        if self.dirty || self.cached_instance_data.is_none() {
            let instances = self.generate_instance_data(neighbors);
            self.cached_instance_data = Some(instances);
            self.dirty = false;
        }
        self.cached_instance_data.as_ref().unwrap().clone() // Return a clone of the cached data
    }


    pub fn generate_instance_data(&self, neighbors: &[Option<&Chunk>; 6]) -> Vec<InstanceData> {
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

    pub(crate) fn is_voxel_solid(&self, x: i32, y: i32, z: i32, neighbors: &[Option<&Chunk>; 6]) -> bool {
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


    pub(crate) fn is_voxel_solid_raycast(&self, x: i32, y: i32, z: i32, neighbors: &[Option<&Chunk>; 6]) -> bool {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 && z >= 0 && z < self.depth as i32 {
            let index = (x + y * self.width as i32 + z * self.width as i32 * self.height as i32) as usize;
            let is_solid = self.voxels[index];
            info!("Checking voxel at local position ({}, {}, {}): {}", x, y, z, is_solid);
            return is_solid;
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




#[derive(Component)]
pub struct ChunkNeedsUpdate;

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

    // Only update if the player has moved to a new chunk
    if new_player_chunk != terrain_state.player_chunk {
        terrain_state.player_chunk = new_player_chunk;

        // Clear the previous update and remove sets
        terrain_state.chunks_to_update.clear();
        terrain_state.chunks_to_remove.clear();

        // Mark chunks for spawning or updating
        for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
            for z in -RENDER_DISTANCE..=RENDER_DISTANCE {
                let current_chunk_position = new_player_chunk + IVec3::new(x, 0, z);
                terrain_state.chunks_to_update.insert(current_chunk_position);
            }
        }

        // Collect positions to remove
        let positions_to_remove: Vec<IVec3> = terrain_state.chunks.keys()
            .filter(|&&pos| (pos - new_player_chunk).abs().max_element() > RENDER_DISTANCE)
            .cloned()
            .collect();

        // Mark chunks for removal
        for pos in positions_to_remove {
            terrain_state.chunks_to_remove.insert(pos);
        }
    }
}

pub fn remove_marked_chunks(
    mut commands: Commands,
    mut terrain_state: ResMut<TerrainState>,
) {
    let chunks_to_remove = terrain_state.chunks_to_remove.clone();

    for &chunk_pos in &chunks_to_remove {
        if let Some(entity) = terrain_state.chunks.remove(&chunk_pos) {
            commands.entity(entity).despawn();
        }
    }

    terrain_state.chunks_to_remove.clear();
}

use bevy::ecs::system::ParamSet;

pub fn update_marked_chunks(
    mut commands: Commands,
    mut terrain_state: ResMut<TerrainState>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let chunks_to_update = terrain_state.chunks_to_update.clone();

    for &chunk_pos in &chunks_to_update {
        if !terrain_state.chunks.contains_key(&chunk_pos) {
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

    terrain_state.chunks_to_update.clear();
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

