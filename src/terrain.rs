use bevy::prelude::*;
use bevy::render::view::NoFrustumCulling;
use noise::{NoiseFn, Perlin};
use crate::cube_mesh::create_cube_mesh;
use crate::{InstanceData, InstanceMaterialData};
use std::collections::HashMap;
use bevy_flycam::FlyCam;

pub const CHUNK_SIZE: i32 = 16;
pub const RENDER_DISTANCE: i32 = 3;

#[derive(Component)]
pub struct TerrainChunk {
    pub position: IVec3,
    width: u32,
    height: u32,
    depth: u32,
    voxels: Vec<bool>,
}

impl TerrainChunk {
    pub fn new(position: IVec3, width: u32, height: u32, depth: u32) -> Self {
        let mut voxels = vec![false; (width * height * depth) as usize];
        let perlin = Perlin::new(0);

        for x in 0..width {
            for z in 0..depth {
                let world_x = position.x * CHUNK_SIZE as i32 + x as i32;
                let world_z = position.z * CHUNK_SIZE as i32 + z as i32;
                let height_value = ((perlin.get([world_x as f64 / 50.0, world_z as f64 / 50.0]) + 1.0) * (height as f64 / 2.0)) as u32;
                for y in 0..height_value {
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
        // Implement normal calculation based on neighboring voxels
        // This is a simplified example; you may want to improve this
        let mut normal = [0.0, 0.0, 0.0];
        if x > 0 && !self.voxels[((x-1) + y * self.width + z * self.width * self.height) as usize] {
            normal[0] -= 1.0;
        }
        if x < self.width - 1 && !self.voxels[((x+1) + y * self.width + z * self.width * self.height) as usize] {
            normal[0] += 1.0;
        }
        // Repeat for y and z axes
        normal
    }


    pub fn create_instance_data(&self) -> Vec<InstanceData> {
        let mut instances = Vec::new();

        for x in 0..self.width {
            for y in 0..self.height {
                for z in 0..self.depth {
                    if self.voxels[(x + y * self.width + z * self.width * self.height) as usize] {
                        let normal = self.calculate_normal(x, y, z);
                        instances.push(InstanceData {
                            position: Vec3::new(
                                (self.position.x * CHUNK_SIZE as i32 + x as i32) as f32,
                                (self.position.y * CHUNK_SIZE as i32 + y as i32) as f32,
                                (self.position.z * CHUNK_SIZE as i32 + z as i32) as f32,
                            ),
                            scale: 1.0,
                            color: [0.5, 0.7, 0.3, 1.0],
                            normal,
                            _padding: 0.0,
                        });
                    }
                }
            }
        }

        instances
    }
}

#[derive(Resource)]
pub struct TerrainState {
    pub chunks: HashMap<IVec3, Entity>,
}

impl Default for TerrainState {
    fn default() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }
}

pub fn update_terrain(
    mut commands: Commands,
    mut terrain_state: ResMut<TerrainState>,
    query: Query<&Transform, With<FlyCam>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let player_transform = query.single();
    let player_chunk = IVec3::new(
        (player_transform.translation.x / CHUNK_SIZE as f32).floor() as i32,
        (player_transform.translation.y / CHUNK_SIZE as f32).floor() as i32,
        (player_transform.translation.z / CHUNK_SIZE as f32).floor() as i32,
    );

    let mut chunks_to_remove = Vec::new();

    // Remove distant chunks
    for (chunk_pos, chunk_entity) in terrain_state.chunks.iter() {
        if (chunk_pos.x - player_chunk.x).abs() > RENDER_DISTANCE
            || (chunk_pos.y - player_chunk.y).abs() > RENDER_DISTANCE
            || (chunk_pos.z - player_chunk.z).abs() > RENDER_DISTANCE
        {
            chunks_to_remove.push(*chunk_pos);
            commands.entity(*chunk_entity).despawn();
        }
    }

    for pos in chunks_to_remove {
        terrain_state.chunks.remove(&pos);
    }

    // Generate new chunks
    for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
        for y in -RENDER_DISTANCE..=RENDER_DISTANCE {
            for z in -RENDER_DISTANCE..=RENDER_DISTANCE {
                let chunk_pos = player_chunk + IVec3::new(x, y, z);
                if !terrain_state.chunks.contains_key(&chunk_pos) {
                    let chunk = TerrainChunk::new(chunk_pos, CHUNK_SIZE as u32, CHUNK_SIZE as u32, CHUNK_SIZE as u32);
                    let instance_data = chunk.create_instance_data();

                    let chunk_entity = commands.spawn((
                        PbrBundle {
                            mesh: meshes.add(create_cube_mesh()),
                            material: materials.add(StandardMaterial {
                                base_color: Color::rgb(0.5, 0.7, 0.3),
                                metallic: 0.0,
                                perceptual_roughness: 0.8,
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        InstanceMaterialData(instance_data),
                        NoFrustumCulling,
                    )).id();

                    terrain_state.chunks.insert(chunk_pos, chunk_entity);
                }
            }
        }
    }
}