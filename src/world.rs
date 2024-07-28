use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use crate::terrain::{Chunk, ChunkMeshingTask};
use std::time::Instant;
use crate::UNLOAD_GRACE_PERIOD;

#[derive(Resource)]
pub struct World {
    pub chunks: HashMap<(i32, i32, i32), Chunk>,
    pub chunk_entities: HashMap<(i32, i32, i32), Entity>,
    pub chunk_size: usize,
    pub render_distance: i32,
    pub last_player_chunk: (i32, i32, i32),
    pub chunk_load_queue: VecDeque<(i32, i32, i32)>,
    pub chunk_unload_queue: VecDeque<(i32, i32, i32)>,
    pub chunk_last_accessed: HashMap<(i32, i32, i32), Instant>,
    pub unload_grace_period: f32,
    pub chunk_loading_queue: Vec<(i32, i32, i32)>
}

impl World {
    pub fn new(chunk_size: usize, render_distance: i32) -> Self {
        Self {
            chunks: HashMap::new(),
            chunk_entities: HashMap::new(),
            chunk_size,
            render_distance,
            last_player_chunk: (0, 0, 0),
            chunk_load_queue: VecDeque::new(),
            chunk_unload_queue: VecDeque::new(),
            chunk_last_accessed: HashMap::new(),
            unload_grace_period: UNLOAD_GRACE_PERIOD,
            chunk_loading_queue: Vec::new(),
        }
    }

    pub fn update_chunks(&mut self, player_chunk_x: i32, player_chunk_y: i32, player_chunk_z: i32) {
        self.last_player_chunk = (player_chunk_x, player_chunk_y, player_chunk_z);
        let now = Instant::now();

        // Clear previous queues
        self.chunk_load_queue.clear();
        self.chunk_unload_queue.clear();

        // Determine chunks to load
        for x in (player_chunk_x - self.render_distance)..=(player_chunk_x + self.render_distance) {
            for y in (player_chunk_y - self.render_distance)..=(player_chunk_y + self.render_distance) {
                for z in (player_chunk_z - self.render_distance)..=(player_chunk_z + self.render_distance) {
                    let chunk_key = (x, y, z);
                    if !self.chunks.contains_key(&chunk_key) {
                        self.chunk_load_queue.push_back(chunk_key);
                    }
                    self.chunk_last_accessed.insert(chunk_key, now);
                }
            }
        }

        // Determine chunks to unload
        let chunks_to_remove: Vec<(i32, i32, i32)> = self.chunks.keys()
            .filter(|&&key| {
                let (x, y, z) = key;
                let distance = ((x - player_chunk_x).pow(2) + (y - player_chunk_y).pow(2) + (z - player_chunk_z).pow(2)) as f32;
                distance > (self.render_distance as f32).powi(2)
            })
            .cloned()
            .collect();

        for chunk_key in chunks_to_remove {
            if let Some(last_accessed) = self.chunk_last_accessed.get(&chunk_key) {
                if now.duration_since(*last_accessed).as_secs_f32() > self.unload_grace_period {
                    self.chunk_unload_queue.push_back(chunk_key);
                }
            }
        }
    }

    pub fn raycast(&self, origin: Vec3, direction: Dir3, max_distance: f32) -> Option<((i32, i32, i32), (usize, usize, usize))> {
        let dir = direction.normalize();
        let step = 0.1; // Smaller step for more precision
        let mut current_pos = origin;

        for _ in 0..((max_distance / step) as i32) {
            let chunk_x = (current_pos.x / self.chunk_size as f32).floor() as i32;
            let chunk_y = (current_pos.y / self.chunk_size as f32).floor() as i32;
            let chunk_z = (current_pos.z / self.chunk_size as f32).floor() as i32;
            let chunk_key = (chunk_x, chunk_y, chunk_z);

            if let Some(chunk) = self.chunks.get(&chunk_key) {
                let local_x = (current_pos.x.rem_euclid(self.chunk_size as f32) as usize).min(self.chunk_size - 1);
                let local_y = (current_pos.y.rem_euclid(self.chunk_size as f32) as usize).min(self.chunk_size - 1);
                let local_z = (current_pos.z.rem_euclid(self.chunk_size as f32) as usize).min(self.chunk_size - 1);

                if chunk.get_voxel(local_x, local_y, local_z) {
                    return Some((chunk_key, (local_x, local_y, local_z)));
                }
            }

            current_pos += dir * step;
        }

        None
    }

    pub fn remove_voxel(&mut self, chunk_key: (i32, i32, i32), voxel_pos: (usize, usize, usize), commands: &mut Commands) {
        println!("Attempting to remove voxel at chunk {:?}, position {:?}", chunk_key, voxel_pos);

        if let Some(chunk) = self.chunks.get_mut(&chunk_key) {
            let (x, y, z) = voxel_pos;
            if chunk.get_voxel(x, y, z) {
                chunk.set_voxel(x, y, z, false);
                println!("Voxel removed successfully");

                // Regenerate mesh for the modified chunk
                if let Some(entity) = self.chunk_entities.get(&chunk_key) {
                    let task = chunk.generate_mesh_task(chunk_key);
                    commands.spawn(task);
                    println!("Mesh regeneration task spawned for chunk {:?}", chunk_key);
                } else {
                    println!("Could not find entity for chunk {:?}", chunk_key);
                }

                // Check and update neighboring chunks if necessary
                self.update_neighboring_chunks(chunk_key, commands);
            } else {
                println!("No voxel found at the specified position");
            }
        } else {
            println!("Chunk not found for key {:?}", chunk_key);
        }
    }

    fn update_neighboring_chunks(&mut self, chunk_key: (i32, i32, i32), commands: &mut Commands) {
        let (cx, cy, cz) = chunk_key;
        let neighbors = [
            (cx - 1, cy, cz), (cx + 1, cy, cz),
            (cx, cy - 1, cz), (cx, cy + 1, cz),
            (cx, cy, cz - 1), (cx, cy, cz + 1),
        ];

        for neighbor_key in neighbors.iter() {
            if let Some(chunk) = self.chunks.get(neighbor_key) {
                let task = chunk.generate_mesh_task(*neighbor_key);
                commands.spawn(task);
                println!("Mesh regeneration task spawned for neighboring chunk {:?}", neighbor_key);
            }
        }
    }

    pub fn process_queue(&mut self, commands: &mut Commands, materials: &mut ResMut<Assets<StandardMaterial>>, meshes: &mut ResMut<Assets<Mesh>>) {
        // Process load queue
        while let Some(chunk_key) = self.chunk_load_queue.pop_front() {
            if !self.chunks.contains_key(&chunk_key) {
                let mut chunk = Chunk::new(self.chunk_size, self.chunk_size, self.chunk_size);
                chunk.generate_terrain(chunk_key.0, chunk_key.1, chunk_key.2);
                self.chunks.insert(chunk_key, chunk);

                let task = self.chunks[&chunk_key].generate_mesh_task(chunk_key);
                commands.spawn(task);
            }
        }

        // Process unload queue
        while let Some(chunk_key) = self.chunk_unload_queue.pop_front() {
            if let Some(entity) = self.chunk_entities.remove(&chunk_key) {
                commands.entity(entity).despawn();
            }
            self.chunks.remove(&chunk_key);
            self.chunk_last_accessed.remove(&chunk_key);
        }
    }

    pub fn spawn_chunk_entity(&mut self, commands: &mut Commands, chunk_key: (i32, i32, i32), mesh: Handle<Mesh>, material: Handle<StandardMaterial>) {
        let chunk_position = Vec3::new(
            chunk_key.0 as f32 * self.chunk_size as f32,
            chunk_key.1 as f32 * self.chunk_size as f32,
            chunk_key.2 as f32 * self.chunk_size as f32,
        );

        let chunk_entity = commands.spawn(PbrBundle {
            mesh,
            material,
            transform: Transform::from_translation(chunk_position),
            ..Default::default()
        }).id();

        self.chunk_entities.insert(chunk_key, chunk_entity);
    }
}