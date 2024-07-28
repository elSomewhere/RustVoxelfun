use bevy::prelude::*;
use bevy::input::mouse::MouseButton;
use bevy_flycam::{FlyCam, PlayerPlugin};
use crate::terrain::{Chunk, ChunkMeshingTask};
use crate::world::{World};
use std::future::Future;
use bevy::tasks::Task;
use std::pin::Pin;
use futures::FutureExt;
use std::task::{Context, Poll};
use std::collections::BinaryHeap;
use std::cmp::Ordering;

mod terrain;
mod world;

pub const CHUNK_SIZE: usize = 16;
pub const RENDER_DISTANCE: i32 = 4;
pub const UNLOAD_GRACE_PERIOD: f32 = 5.0; // seconds
pub const VOXEL_REMOVAL_RANGE: f32 = 20.0; // Increased from 5.0 to 20.0

#[derive(Component)]
struct CameraLight;

#[derive(Component)]
struct VoxelRemover;

use bevy_flycam::NoCameraPlayerPlugin;

#[derive(Clone, Eq, PartialEq)]
struct PrioritizedChunk {
    distance: i32,
    chunk_key: (i32, i32, i32),
}

impl Ord for PrioritizedChunk {
    fn cmp(&self, other: &Self) -> Ordering {
        other.distance.cmp(&self.distance)
    }
}

impl PartialOrd for PrioritizedChunk {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(NoCameraPlayerPlugin)
        .insert_resource(World::new(CHUNK_SIZE, RENDER_DISTANCE))
        .add_systems(Startup, setup)
        .add_systems(Update, (
            update_chunks,
            process_chunk_queue,
            sync_light_with_camera,
            handle_meshing_tasks,
            voxel_removal_system,
            prioritize_chunks,
        ))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Enhanced lighting with CameraLight component
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 10000.0,
            shadows_enabled: true,
            range: 500.0,
            ..Default::default()
        },
        transform: Transform::from_xyz(10.0, 20.0, 10.0),
        ..Default::default()
    }).insert(CameraLight);

    // Spawn a new camera with FlyCam and VoxelRemover components
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 2.0, 0.5),
            ..default()
        },
        FlyCam,
        VoxelRemover
    ));
}

fn update_chunks(
    mut world: ResMut<World>,
    query: Query<&Transform, With<Camera>>,
    time: Res<Time>,
) {
    if let Ok(player_transform) = query.get_single() {
        let player_chunk_x = (player_transform.translation.x / world.chunk_size as f32).floor() as i32;
        let player_chunk_y = (player_transform.translation.y / world.chunk_size as f32).floor() as i32;
        let player_chunk_z = (player_transform.translation.z / world.chunk_size as f32).floor() as i32;

        let current_chunk = (player_chunk_x, player_chunk_y, player_chunk_z);
        world.update_chunks(current_chunk.0, current_chunk.1, current_chunk.2);
    }
}

fn prioritize_chunks(
    mut world: ResMut<World>,
    query: Query<&Transform, With<Camera>>,
) {
    if let Ok(player_transform) = query.get_single() {
        let player_chunk_x = (player_transform.translation.x / world.chunk_size as f32).floor() as i32;
        let player_chunk_y = (player_transform.translation.y / world.chunk_size as f32).floor() as i32;
        let player_chunk_z = (player_transform.translation.z / world.chunk_size as f32).floor() as i32;

        let mut priority_queue = BinaryHeap::new();

        for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
            for y in -RENDER_DISTANCE..=RENDER_DISTANCE {
                for z in -RENDER_DISTANCE..=RENDER_DISTANCE {
                    let chunk_key = (player_chunk_x + x, player_chunk_y + y, player_chunk_z + z);
                    let distance = x.abs() + y.abs() + z.abs();

                    priority_queue.push(PrioritizedChunk {
                        distance,
                        chunk_key,
                    });
                }
            }
        }

        // Update the world's chunk loading queue with the prioritized chunks
        world.chunk_loading_queue = priority_queue.into_iter().map(|pc| pc.chunk_key).collect();
    }
}

fn process_chunk_queue(
    mut world: ResMut<World>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    world.process_queue(&mut commands, &mut materials, &mut meshes);
}

fn sync_light_with_camera(
    mut param_set: ParamSet<(
        Query<&Transform, With<Camera>>,
        Query<&mut Transform, (With<PointLight>, With<CameraLight>)>,
    )>,
) {
    let camera_translation = {
        if let Ok(camera_transform) = param_set.p0().get_single() {
            Some(camera_transform.translation)
        } else {
            None
        }
    };

    if let Some(translation) = camera_translation {
        if let Ok(mut light_transform) = param_set.p1().get_single_mut() {
            light_transform.translation = translation;
        }
    }
}

fn handle_meshing_tasks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshing_tasks: Query<(Entity, &mut ChunkMeshingTask)>,
    mut world: ResMut<World>,
    mut chunk_entities: Query<(Entity, &mut Handle<Mesh>), With<Chunk>>,
) {
    let mut context = Context::from_waker(futures::task::noop_waker_ref());

    for (entity, mut task) in &mut meshing_tasks {
        if let Poll::Ready(mesh) = Pin::new(&mut task.0).poll_unpin(&mut context) {
            let chunk_key = task.1;

            // Check if the chunk entity already exists
            if let Some(existing_entity) = world.chunk_entities.get(&chunk_key) {
                // Update existing chunk entity
                if let Ok((_, mut mesh_handle)) = chunk_entities.get_mut(*existing_entity) {
                    // Update the mesh
                    *mesh_handle = meshes.add(mesh);
                }
            } else {
                // Create new chunk entity
                let material = materials.add(StandardMaterial {
                    base_color: Color::rgb(0.8, 0.7, 0.6),
                    ..default()
                });
                let mesh_handle = meshes.add(mesh);

                let chunk_entity = commands.spawn((
                    PbrBundle {
                        mesh: mesh_handle,
                        material: material.clone(),
                        transform: Transform::from_xyz(
                            (chunk_key.0 * CHUNK_SIZE as i32) as f32,
                            (chunk_key.1 * CHUNK_SIZE as i32) as f32,
                            (chunk_key.2 * CHUNK_SIZE as i32) as f32
                        ),
                        ..default()
                    },
                    Chunk::new(CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE), // Assuming Chunk::new takes dimensions
                ))
                    .id();

                // Store the new entity in the world
                world.chunk_entities.insert(chunk_key, chunk_entity);
            }

            // Remove the meshing task entity
            commands.entity(entity).despawn();
        }
    }
}

fn voxel_removal_system(
    mut world: ResMut<World>,
    camera_query: Query<&Transform, With<VoxelRemover>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        if let Ok(camera_transform) = camera_query.get_single() {
            let ray_origin = camera_transform.translation;
            let ray_direction = camera_transform.forward();

            println!("Attempting to remove voxel. Ray origin: {:?}, direction: {:?}", ray_origin, ray_direction);

            // Debug: Print chunk and voxel at camera position
            let chunk_x = (ray_origin.x / world.chunk_size as f32).floor() as i32;
            let chunk_y = (ray_origin.y / world.chunk_size as f32).floor() as i32;
            let chunk_z = (ray_origin.z / world.chunk_size as f32).floor() as i32;
            println!("Camera is in chunk: ({}, {}, {})", chunk_x, chunk_y, chunk_z);

            if let Some((chunk_key, voxel_pos)) = world.raycast(ray_origin, ray_direction, VOXEL_REMOVAL_RANGE) {
                println!("Raycast hit: Chunk {:?}, Voxel position {:?}", chunk_key, voxel_pos);
                world.remove_voxel(chunk_key, voxel_pos, &mut commands);
            } else {
                println!("Raycast did not hit any voxel within range of {}", VOXEL_REMOVAL_RANGE);

                // Debug: Check voxels along the ray
                let mut current_pos = ray_origin;
                let steps = (VOXEL_REMOVAL_RANGE / 0.1) as i32; // Adjust step size if needed
                for i in 0..steps {
                    let chunk_x = (current_pos.x / world.chunk_size as f32).floor() as i32;
                    let chunk_y = (current_pos.y / world.chunk_size as f32).floor() as i32;
                    let chunk_z = (current_pos.z / world.chunk_size as f32).floor() as i32;
                    let chunk_key = (chunk_x, chunk_y, chunk_z);

                    if let Some(chunk) = world.chunks.get(&chunk_key) {
                        let local_x = (current_pos.x.rem_euclid(world.chunk_size as f32) as usize).min(world.chunk_size - 1);
                        let local_y = (current_pos.y.rem_euclid(world.chunk_size as f32) as usize).min(world.chunk_size - 1);
                        let local_z = (current_pos.z.rem_euclid(world.chunk_size as f32) as usize).min(world.chunk_size - 1);

                        if chunk.get_voxel(local_x, local_y, local_z) {
                            println!("Found voxel at step {}: Chunk {:?}, Local pos ({}, {}, {})", i, chunk_key, local_x, local_y, local_z);
                            break;
                        }
                    }

                    current_pos += ray_direction * 0.1;
                }
            }
        } else {
            println!("Could not find camera transform");
        }
    }
}