use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct InstanceData {
    pub position: Vec3,
    pub scale: f32,
    pub color: [f32; 4],
    pub normal: [f32; 3],
    pub _padding: f32, // To ensure 16-byte alignment
}