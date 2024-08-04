use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use noise::{NoiseFn, Perlin};
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
            dirty: true, // New chunk is initially dirty
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

    pub fn is_voxel_solid(&self, x: i32, y: i32, z: i32) -> bool {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 && z >= 0 && z < self.depth as i32 {
            self.voxels[(x + y * self.width as i32 + z * self.width as i32 * self.height as i32) as usize]
        } else {
            false
        }
    }
}



pub fn prepare_chunk_updates(
    terrain_state: Res<TerrainState>,
    mut chunk_query: Query<&mut Chunk>,
) {
    for &chunk_pos in &terrain_state.chunks_to_update {
        if let Some(&entity) = terrain_state.chunks.get(&chunk_pos) {
            if let Ok(mut chunk) = chunk_query.get_mut(entity) {
                chunk.dirty = true;
            }
        }
    }
}

pub fn apply_chunk_updates(
    mut chunk_query: Query<(&mut Chunk, &mut Handle<Mesh>)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (mut chunk, mut mesh_handle) in chunk_query.iter_mut() {
        if chunk.dirty {
            info!("Applying update to chunk at position: {:?}", chunk.position);
            let new_mesh = create_chunk_mesh(&chunk);
            *mesh_handle = meshes.add(new_mesh);
            chunk.dirty = false;
        }
    }
}

pub fn create_chunk_mesh(chunk: &Chunk) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for x in 0..chunk.width {
        for y in 0..chunk.height {
            for z in 0..chunk.depth {
                if chunk.is_voxel_solid(x as i32, y as i32, z as i32) {
                    add_voxel_to_mesh(x as f32, y as f32, z as f32, &mut positions, &mut normals, &mut uvs, &mut indices);
                }
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn add_voxel_to_mesh(x: f32, y: f32, z: f32, positions: &mut Vec<[f32; 3]>, normals: &mut Vec<[f32; 3]>, uvs: &mut Vec<[f32; 2]>, indices: &mut Vec<u32>) {
    let voxel_positions = [
        // Front face
        [x, y, z], [x + 1.0, y, z], [x + 1.0, y + 1.0, z], [x, y + 1.0, z],
        // Back face
        [x, y, z + 1.0], [x + 1.0, y, z + 1.0], [x + 1.0, y + 1.0, z + 1.0], [x, y + 1.0, z + 1.0],
        // Right face
        [x + 1.0, y, z], [x + 1.0, y, z + 1.0], [x + 1.0, y + 1.0, z + 1.0], [x + 1.0, y + 1.0, z],
        // Left face
        [x, y, z], [x, y, z + 1.0], [x, y + 1.0, z + 1.0], [x, y + 1.0, z],
        // Top face
        [x, y + 1.0, z], [x + 1.0, y + 1.0, z], [x + 1.0, y + 1.0, z + 1.0], [x, y + 1.0, z + 1.0],
        // Bottom face
        [x, y, z], [x + 1.0, y, z], [x + 1.0, y, z + 1.0], [x, y, z + 1.0],
    ];

    let face_normals = [
        [0.0, 0.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, -1.0, 0.0],
    ];

    let face_uvs = [
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
    ];

    for face in 0..6 {
        let offset = positions.len() as u32;
        positions.extend_from_slice(&voxel_positions[face * 4..(face + 1) * 4]);
        normals.extend_from_slice(&[face_normals[face]; 4]);
        uvs.extend_from_slice(&face_uvs);
        indices.extend_from_slice(&[offset, offset + 1, offset + 2, offset + 2, offset + 3, offset]);
    }
}


fn create_voxel_mesh(x: f32, y: f32, z: f32) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<u32>) {
    // Define vertices, normals, and indices for a unit cube
    // Translate the vertices by (x, y, z)
    // Return the vertices, normals, and indices
    // This is a simplified example and should be optimized for real use
    let positions = vec![
        [x, y, z], [x + 1.0, y, z], [x + 1.0, y + 1.0, z], [x, y + 1.0, z],
        [x, y, z + 1.0], [x + 1.0, y, z + 1.0], [x + 1.0, y + 1.0, z + 1.0], [x, y + 1.0, z + 1.0],
    ];
    let normals = vec![
        [-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0], [0.0, 0.0, -1.0], [0.0, 0.0, 1.0],
    ];
    let indices = vec![
        0, 1, 2, 2, 3, 0, // Front face
        1, 5, 6, 6, 2, 1, // Right face
        5, 4, 7, 7, 6, 5, // Back face
        4, 0, 3, 3, 7, 4, // Left face
        3, 2, 6, 6, 7, 3, // Top face
        4, 5, 1, 1, 0, 4, // Bottom face
    ];
    (positions, normals, indices)
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