use bevy::prelude::{Assets, Color, Commands, Mesh, PbrBundle, ResMut, StandardMaterial, Vec3};
use bevy::render::view::NoFrustumCulling;
use noise::{NoiseFn, Perlin};
use crate::cube_mesh::create_cube_mesh;
use crate::{InstanceData, InstanceMaterialData};

pub struct TerrainChunk {
    width: u32,
    height: u32,
    depth: u32,
    voxels: Vec<bool>,
}

impl TerrainChunk {
    pub fn new(width: u32, height: u32, depth: u32) -> Self {
        let mut voxels = vec![false; (width * height * depth) as usize];
        let perlin = Perlin::new(0);

        for x in 0..width {
            for z in 0..depth {
                let height_value = ((perlin.get([x as f64 / 10.0, z as f64 / 10.0]) + 1.0) * (height as f64 / 2.0)) as u32;
                for y in 0..height_value {
                    voxels[(x + y * width + z * width * height) as usize] = true;
                }
            }
        }

        Self {
            width,
            height,
            depth,
            voxels,
        }
    }

    pub fn create_instance_data(&self) -> Vec<Instance> {
        let mut instances = Vec::new();

        for x in 0..self.width {
            for y in 0..self.height {
                for z in 0..self.depth {
                    if self.voxels[(x + y * self.width + z * self.width * self.height) as usize] {
                        instances.push(Instance {
                            position: [x as f32, y as f32, z as f32],
                        });
                    }
                }
            }
        }

        instances
    }

    pub fn generate_terrain(&self, commands: &mut Commands, mut materials: ResMut<Assets<StandardMaterial>>, mut meshes: ResMut<Assets<Mesh>>) {
        let instances = self.create_instance_data();
        let instance_data = instances.iter().map(|instance| {
            InstanceData {
                position: Vec3::new(instance.position[0], instance.position[1], instance.position[2]),
                scale: 1.0,
                color: [0.3, 0.6, 0.3, 1.0], // Greenish color for terrain
            }
        }).collect::<Vec<_>>();

        commands.spawn((
            PbrBundle {
                mesh: meshes.add(create_cube_mesh()),
                material: materials.add(StandardMaterial {
                    base_color: Color::rgb(0.3, 0.6, 0.3),
                    ..Default::default()
                }),
                ..Default::default()
            },
            InstanceMaterialData(instance_data),
            NoFrustumCulling,
        ));
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Instance {
    pub position: [f32; 3],
}

unsafe impl bytemuck::Pod for Instance {}
unsafe impl bytemuck::Zeroable for Instance {}
