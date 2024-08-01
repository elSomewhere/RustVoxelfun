use bevy::prelude::*;
use bevy::render::view::NoFrustumCulling;
use noise::{NoiseFn, Perlin};
use crate::terrain::Terrain;

use crate::cube_mesh::create_cube_mesh;
use crate::{InstanceData, InstanceMaterialData};
use std::collections::HashMap;
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

    fn calculate_normal(&self, x: u32, y: u32, z: u32) -> [f32; 3] {
        let mut normal = [0.0, 0.0, 0.0];
        if x > 0 && !self.voxels[((x-1) + y * self.width + z * self.width * self.height) as usize] {
            normal[0] -= 1.0;
        }
        if x < self.width - 1 && !self.voxels[((x+1) + y * self.width + z * self.width * self.height) as usize] {
            normal[0] += 1.0;
        }
        if y > 0 && !self.voxels[(x + (y-1) * self.width + z * self.width * self.height) as usize] {
            normal[1] -= 1.0;
        }
        if y < self.height - 1 && !self.voxels[(x + (y+1) * self.width + z * self.width * self.height) as usize] {
            normal[1] += 1.0;
        }
        if z > 0 && !self.voxels[(x + y * self.width + (z-1) * self.width * self.height) as usize] {
            normal[2] -= 1.0;
        }
        if z < self.depth - 1 && !self.voxels[(x + y * self.width + (z+1) * self.width * self.height) as usize] {
            normal[2] += 1.0;
        }
        normal
    }

    pub fn create_instance_data(&self) -> Vec<InstanceData> {
        let mut instances = Vec::new();

        for x in 0..self.width {
            for y in 0..self.height {
                for z in 0..self.depth {
                    if self.voxels[(x + y * self.width + z * self.width * self.height) as usize] {
                        let world_x = self.position.x * CHUNK_SIZE as i32 + x as i32;
                        let world_y = self.position.y * CHUNK_SIZE as i32 + y as i32;
                        let world_z = self.position.z * CHUNK_SIZE as i32 + z as i32;

                        // Add all voxels that are not completely surrounded
                        if self.is_visible(x, y, z) {
                            let normal = self.calculate_normal(x, y, z);
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

    fn is_visible(&self, x: u32, y: u32, z: u32) -> bool {
        let neighbors = [
            (x.wrapping_sub(1), y, z),
            (x.wrapping_add(1), y, z),
            (x, y.wrapping_sub(1), z),
            (x, y.wrapping_add(1), z),
            (x, y, z.wrapping_sub(1)),
            (x, y, z.wrapping_add(1)),
        ];

        neighbors.iter().any(|&(nx, ny, nz)| {
            nx >= self.width || ny >= self.height || nz >= self.depth ||
                !self.voxels[(nx + ny * self.width + nz * self.width * self.height) as usize]
        })
    }
}

#[derive(Resource, Default)]
pub struct TerrainState {
    pub chunks: HashMap<IVec3, Entity>,
}

pub fn update_terrain(
    mut commands: Commands,
    mut terrain_state: ResMut<Terrain>,
    query: Query<&Transform, With<FlyCam>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let player_position = query.single().translation;
    let chunk_position = IVec3::new(
        (player_position.x / (CHUNK_SIZE as f32 * VOXEL_SIZE)).floor() as i32,
        0,
        (player_position.z / (CHUNK_SIZE as f32 * VOXEL_SIZE)).floor() as i32,
    );

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
                let instance_data = chunk.create_instance_data();
                let chunk_entity = commands.spawn((
                    meshes.add(create_cube_mesh()),
                    SpatialBundle::INHERITED_IDENTITY,
                    InstanceMaterialData(instance_data),
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
}