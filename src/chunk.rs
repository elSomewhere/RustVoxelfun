use std::collections::HashMap;
use bevy::prelude::*;
use bevy::render::view::NoFrustumCulling;
use noise::{NoiseFn, Perlin};

use crate::cube_mesh::create_cube_mesh;
use bevy_flycam::FlyCam;
use crate::resources::{InstanceMaterialData, VoxelResources};
use crate::terrain::TerrainState;

pub const CHUNK_SIZE: i32 = 4;
pub const RENDER_DISTANCE: i32 = 4;
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
            dirty: true,
        }
    }

    pub fn create_voxel_entities(&self, commands: &mut Commands, mesh: Handle<Mesh>, material: Handle<StandardMaterial>) -> Entity {
        let mut instances = Vec::new();

        for x in 0..self.width {
            for y in 0..self.height {
                for z in 0..self.depth {
                    let index = (x + y * self.width + z * self.width * self.height) as usize;
                    if self.voxels[index] {
                        let world_x = self.position.x * CHUNK_SIZE as i32 + x as i32;
                        let world_y = self.position.y * CHUNK_SIZE as i32 + y as i32;
                        let world_z = self.position.z * CHUNK_SIZE as i32 + z as i32;

                        instances.push(Transform::from_xyz(
                            world_x as f32 * VOXEL_SIZE,
                            world_y as f32 * VOXEL_SIZE,
                            world_z as f32 * VOXEL_SIZE,
                        ).with_scale(Vec3::splat(VOXEL_SIZE)));
                    }
                }
            }
        }

        commands.spawn((
            MaterialMeshBundle {
                mesh,
                material,
                transform: Transform::from_xyz(
                    self.position.x as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE,
                    self.position.y as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE,
                    self.position.z as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE,
                ),
                ..default()
            },
            InstanceMaterialData(instances),
        )).id()
    }

    pub fn update_voxel_entities(&self, commands: &mut Commands, entity: Entity) {
        let mut instances = Vec::new();

        for x in 0..self.width {
            for y in 0..self.height {
                for z in 0..self.depth {
                    let index = (x + y * self.width + z * self.width * self.height) as usize;
                    if self.voxels[index] {
                        let world_x = self.position.x * CHUNK_SIZE as i32 + x as i32;
                        let world_y = self.position.y * CHUNK_SIZE as i32 + y as i32;
                        let world_z = self.position.z * CHUNK_SIZE as i32 + z as i32;

                        instances.push(Transform::from_xyz(
                            world_x as f32 * VOXEL_SIZE,
                            world_y as f32 * VOXEL_SIZE,
                            world_z as f32 * VOXEL_SIZE,
                        ).with_scale(Vec3::splat(VOXEL_SIZE)));
                    }
                }
            }
        }

        commands.entity(entity).insert(InstanceMaterialData(instances));
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

pub fn remove_marked_chunks(
    mut commands: Commands,
    mut terrain_state: ResMut<TerrainState>,
) {
    let chunks_to_remove = terrain_state.chunks_to_remove.clone();

    for &chunk_pos in &chunks_to_remove {
        if let Some(entity) = terrain_state.chunks.remove(&chunk_pos) {
            info!("Removing chunk at position: {:?}", chunk_pos);
            commands.entity(entity).despawn();
        }
    }

    terrain_state.chunks_to_remove.clear();
}
pub fn prepare_chunk_updates(
    terrain_state: Res<TerrainState>,
    chunk_query: Query<&Chunk>,
    mut commands: Commands,
    voxel_resources: Res<VoxelResources>,
) {
    for &chunk_pos in &terrain_state.chunks_to_update {
        if let Some(&entity) = terrain_state.chunks.get(&chunk_pos) {
            if let Ok(chunk) = chunk_query.get(entity) {
                if chunk.dirty {
                    commands.entity(entity).insert(PreparedChunkUpdate {
                        mesh: voxel_resources.mesh.clone(),
                        material: voxel_resources.material.clone(),
                    });
                }
            }
        }
    }
}


#[derive(Component)]
pub struct PreparedChunkUpdate {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

pub fn apply_chunk_updates(
    mut commands: Commands,
    mut chunk_query: Query<(Entity, &mut Chunk, Option<&PreparedChunkUpdate>)>,
) {
    for (entity, mut chunk, prepared_update) in chunk_query.iter_mut() {
        if prepared_update.is_some() {
            info!("Applying update to chunk at position: {:?}", chunk.position);
            chunk.update_voxel_entities(&mut commands, entity);
            commands.entity(entity).remove::<PreparedChunkUpdate>();
        }
    }
}

