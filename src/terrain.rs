use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use crate::chunk::Chunk;

#[derive(Resource, Default)]
pub struct TerrainState {
    pub chunks: HashMap<IVec3, Entity>,
    pub chunks_to_update: HashSet<IVec3>,
    pub chunks_to_remove: HashSet<IVec3>,
}