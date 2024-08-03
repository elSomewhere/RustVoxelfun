use bevy::{
    core_pipeline::core_3d::Transparent3d,
    ecs::{
        query::QueryItem,
        system::{lifetimeless::*, SystemParamItem},
    },
    pbr::{
        MeshPipeline, MeshPipelineKey, RenderMeshInstances, SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{GpuBufferInfo, GpuMesh, MeshVertexBufferLayoutRef},
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::*,
        renderer::RenderDevice,
        view::{ExtractedView, NoFrustumCulling},
        Render, RenderApp, RenderSet,
    },
};
use bytemuck::{Pod, Zeroable};

mod cube_mesh;
mod terrain;
mod chunk;
mod interaction;
mod rendering;
mod world;
mod types;

use bevy_flycam::prelude::*;
use crate::chunk::{apply_chunk_updates, prepare_chunk_updates, remove_marked_chunks};
use crate::interaction::handle_mouse_input;
use crate::rendering::CustomMaterialPlugin;
use crate::terrain::TerrainState;
use crate::world::{mark_chunks_for_update, update_marked_chunks};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PlayerPlugin)
        .add_plugins((
            CustomMaterialPlugin,
        ))
        .insert_resource(TerrainState::default()) // Add this line
        .add_systems(Startup, setup_lighting)
        .add_systems(Update, mark_chunks_for_update)
        .add_systems(Update, update_marked_chunks)
        .add_systems(Update, prepare_chunk_updates)
        .add_systems(Update, apply_chunk_updates)
        .add_systems(Update, remove_marked_chunks)
        .add_systems(Update, handle_mouse_input)
        .run();
}



fn setup_lighting(mut commands: Commands) {
    // Add a directional light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 100.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4),
            ..default()
        },
        ..default()
    });

    // Add an ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.2,
    });
}