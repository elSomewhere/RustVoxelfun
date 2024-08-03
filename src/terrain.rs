use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use crate::chunk::Chunk;

#[derive(Resource)]
pub struct TerrainState {
    pub chunks: HashMap<IVec3, Entity>,
    pub chunks_to_update: HashSet<IVec3>,
    pub chunks_to_remove: HashSet<IVec3>,
    pub player_chunk: IVec3,
}

impl Default for TerrainState {
    fn default() -> Self {
        Self {
            chunks: HashMap::new(),
            chunks_to_update: HashSet::new(),
            chunks_to_remove: HashSet::new(),
            player_chunk: IVec3::ZERO,
        }
    }
}