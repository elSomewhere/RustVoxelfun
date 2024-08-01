use bevy::prelude::*;
use std::collections::HashMap;
use crate::chunk::Chunk;

#[derive(Resource, Default)]
pub struct Terrain {
    pub chunks: HashMap<IVec3, Entity>,
}