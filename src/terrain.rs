use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bytemuck::{Pod, Zeroable};
use std::collections::{HashMap, HashSet, VecDeque};
use bevy::render::mesh::{Indices, PrimitiveTopology};
use noise::{NoiseFn, Perlin};
use std::sync::{Arc, Mutex};
use crate::UNLOAD_GRACE_PERIOD;

#[derive(Component)]
pub struct Chunk {
    pub voxels: Vec<bool>,
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    pub last_accessed: f32,
    pub boxified: Vec<bool>,
}

#[derive(Component)]
pub struct ChunkMeshingTask(pub Task<Mesh>, pub (i32, i32, i32));

impl Chunk {

    pub fn generate_mesh_task(&self, chunk_key: (i32, i32, i32)) -> ChunkMeshingTask {
        let voxels = self.voxels.clone();
        let width = self.width;
        let height = self.height;
        let depth = self.depth;

        let task = AsyncComputeTaskPool::get().spawn(async move {
            let mut chunk = Chunk {
                voxels,
                width,
                height,
                depth,
                last_accessed: 0.0,
                boxified: vec![false; width * height * depth],
            };
            chunk.generate_mesh()
        });

        ChunkMeshingTask(task, chunk_key)
    }



    pub fn generate_terrain(&mut self, chunk_x: i32, chunk_y: i32, chunk_z: i32) {
        let perlin = Perlin::new(0);
        let chunk_size = self.width as f64;

        for x in 0..self.width {
            for z in 0..self.depth {
                let world_x = chunk_x as f64 * chunk_size + x as f64;
                let world_z = chunk_z as f64 * chunk_size + z as f64;

                // Adjust these values to change the terrain characteristics
                let frequency = 0.01;
                let amplitude = 32.0;

                let height = ((perlin.get([world_x * frequency, world_z * frequency]) + 1.0) * 0.5 * amplitude) as usize;

                for y in 0..self.height {
                    let world_y = if chunk_y >= 0 {
                        (chunk_y as usize * self.height) + y
                    } else {
                        y.saturating_sub(chunk_y.unsigned_abs() as usize * self.height)
                    };
                    if world_y < self.height {
                        if world_y < height {
                            self.set_voxel(x, y, z, true);
                        } else {
                            self.set_voxel(x, y, z, false);
                        }
                    }
                }
            }
        }
    }


    pub fn new(width: usize, height: usize, depth: usize) -> Self {
        let voxels = vec![false; width * height * depth];
        let boxified = vec![false; width * height * depth];
        Self { voxels, width, height, depth, last_accessed: 0.0, boxified }
    }

    pub fn get_voxel(&self, x: usize, y: usize, z: usize) -> bool {
        if x < self.width && y < self.height && z < self.depth {
            self.voxels[x + y * self.width + z * self.width * self.height]
        } else {
            false
        }
    }

    pub fn set_voxel(&mut self, x: usize, y: usize, z: usize, value: bool) {
        if x < self.width && y < self.height && z < self.depth {
            self.voxels[x + y * self.width + z * self.width * self.height] = value;
        }
    }

    fn get_box_index(&self, x: usize, y: usize, z: usize) -> usize {
        x + y * self.width + z * self.width * self.height
    }

    fn is_boxified(&self, x: usize, y: usize, z: usize) -> bool {
        self.boxified[self.get_box_index(x, y, z)]
    }

    fn set_boxified(&mut self, x: usize, y: usize, z: usize, value: bool) {
        let index: usize = self.get_box_index(x, y, z);
        self.boxified[index] = value;
    }

    pub fn merge_voxels(&mut self) -> Vec<(usize, usize, usize, usize, usize, usize)> {
        let mut boxes = Vec::new();
        self.boxified.fill(false);

        for x in 0..self.width {
            for y in 0..self.height {
                for z in 0..self.depth {
                    if self.get_voxel(x, y, z) && !self.is_boxified(x, y, z) {
                        let mut nx = 1;
                        let mut ny = 1;
                        let mut nz = 1;

                        // Merge in x direction
                        for i in x + 1..self.width {
                            if self.get_voxel(i, y, z) && !self.is_boxified(i, y, z) {
                                nx += 1;
                            } else {
                                break;
                            }
                        }

                        // Merge in y direction
                        for j in y + 1..self.height {
                            let mut valid = true;
                            for i in x..x + nx {
                                if !self.get_voxel(i, j, z) || self.is_boxified(i, j, z) {
                                    valid = false;
                                    break;
                                }
                            }
                            if valid {
                                ny += 1;
                            } else {
                                break;
                            }
                        }

                        // Merge in z direction
                        for k in z + 1..self.depth {
                            let mut valid = true;
                            for i in x..x + nx {
                                for j in y..y + ny {
                                    if !self.get_voxel(i, j, k) || self.is_boxified(i, j, k) {
                                        valid = false;
                                        break;
                                    }
                                }
                                if !valid {
                                    break;
                                }
                            }
                            if valid {
                                nz += 1;
                            } else {
                                break;
                            }
                        }

                        // Mark voxels as boxified
                        for i in x..x + nx {
                            for j in y..y + ny {
                                for k in z..z + nz {
                                    self.set_boxified(i, j, k, true);
                                }
                            }
                        }

                        boxes.push((x, y, z, nx, ny, nz));
                    }
                }
            }
        }
        boxes
    }

    pub fn generate_mesh(&mut self) -> Mesh {
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        let mut normals = Vec::new();
        let mut uvs: Vec<[f32; 2]> = Vec::new();

        let mut index_count = 0;
        for (x, y, z, nx, ny, nz) in self.merge_voxels() {
            // Vertices for each face of the box
            let face_vertices = [
                // Front face
                [x as f32, y as f32, z as f32],
                [(x + nx) as f32, y as f32, z as f32],
                [x as f32, (y + ny) as f32, z as f32],
                [(x + nx) as f32, (y + ny) as f32, z as f32],

                // Back face
                [x as f32, y as f32, (z + nz) as f32],
                [(x + nx) as f32, y as f32, (z + nz) as f32],
                [x as f32, (y + ny) as f32, (z + nz) as f32],
                [(x + nx) as f32, (y + ny) as f32, (z + nz) as f32],

                // Left face
                [x as f32, y as f32, z as f32],
                [x as f32, (y + ny) as f32, z as f32],
                [x as f32, y as f32, (z + nz) as f32],
                [x as f32, (y + ny) as f32, (z + nz) as f32],

                // Right face
                [(x + nx) as f32, y as f32, z as f32],
                [(x + nx) as f32, (y + ny) as f32, z as f32],
                [(x + nx) as f32, y as f32, (z + nz) as f32],
                [(x + nx) as f32, (y + ny) as f32, (z + nz) as f32],

                // Top face
                [x as f32, (y + ny) as f32, z as f32],
                [(x + nx) as f32, (y + ny) as f32, z as f32],
                [x as f32, (y + ny) as f32, (z + nz) as f32],
                [(x + nx) as f32, (y + ny) as f32, (z + nz) as f32],

                // Bottom face
                [x as f32, y as f32, z as f32],
                [(x + nx) as f32, y as f32, z as f32],
                [x as f32, y as f32, (z + nz) as f32],
                [(x + nx) as f32, y as f32, (z + nz) as f32],
            ];

            // Indices for each face of the box
            let face_indices = [
                // Correct winding order for each face
                0, 2, 1, 2, 3, 1,   // Front face
                4, 5, 6, 5, 7, 6,   // Back face
                8, 10, 9, 10, 11, 9, // Left face
                12, 13, 14, 13, 15, 14, // Right face
                16, 18, 17, 18, 19, 17, // Top face
                20, 21, 22, 21, 23, 22, // Bottom face
            ];

            // Normals for each face of the box
            let face_normals = [
                [0.0f32, 0.0f32, -1.0f32], // Front face
                [0.0f32, 0.0f32, -1.0f32],
                [0.0f32, 0.0f32, -1.0f32],
                [0.0f32, 0.0f32, -1.0f32],

                [0.0f32, 0.0f32, 1.0f32],  // Back face
                [0.0f32, 0.0f32, 1.0f32],
                [0.0f32, 0.0f32, 1.0f32],
                [0.0f32, 0.0f32, 1.0f32],

                [-1.0f32, 0.0f32, 0.0f32], // Left face
                [-1.0f32, 0.0f32, 0.0f32],
                [-1.0f32, 0.0f32, 0.0f32],
                [-1.0f32, 0.0f32, 0.0f32],

                [1.0f32, 0.0f32, 0.0f32],  // Right face
                [1.0f32, 0.0f32, 0.0f32],
                [1.0f32, 0.0f32, 0.0f32],
                [1.0f32, 0.0f32, 0.0f32],

                [0.0f32, 1.0f32, 0.0f32],  // Top face
                [0.0f32, 1.0f32, 0.0f32],
                [0.0f32, 1.0f32, 0.0f32],
                [0.0f32, 1.0f32, 0.0f32],

                [0.0f32, -1.0f32, 0.0f32], // Bottom face
                [0.0f32, -1.0f32, 0.0f32],
                [0.0f32, -1.0f32, 0.0f32],
                [0.0f32, -1.0f32, 0.0f32],
            ];

            // UV coordinates for each face of the box
            let face_uvs = [
                [0.0f32, 0.0f32], [1.0f32, 0.0f32], [0.0f32, 1.0f32], [1.0f32, 1.0f32], // Front face
                [0.0f32, 0.0f32], [1.0f32, 0.0f32], [0.0f32, 1.0f32], [1.0f32, 1.0f32], // Back face
                [0.0f32, 0.0f32], [1.0f32, 0.0f32], [0.0f32, 1.0f32], [1.0f32, 1.0f32], // Left face
                [0.0f32, 0.0f32], [1.0f32, 0.0f32], [0.0f32, 1.0f32], [1.0f32, 1.0f32], // Right face
                [0.0f32, 0.0f32], [1.0f32, 0.0f32], [0.0f32, 1.0f32], [1.0f32, 1.0f32], // Top face
                [0.0f32, 0.0f32], [1.0f32, 0.0f32], [0.0f32, 1.0f32], [1.0f32, 1.0f32], // Bottom face
            ];

            positions.extend_from_slice(&face_vertices);
            indices.extend(face_indices.iter().map(|&i| i + index_count));
            normals.extend_from_slice(&face_normals);
            uvs.extend_from_slice(&face_uvs);

            index_count += 24; // 24 vertices per voxel
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(Indices::U32(indices));

        mesh
    }
}