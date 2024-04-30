use std::borrow::Cow;

use bevy::{
  ecs::entity::EntityHashMap,
  prelude::*,
  render::{
    render_graph::Node,
    render_resource::{
      binding_types::{
        storage_buffer, storage_buffer_read_only, uniform_buffer,
      },
      BindGroupLayout, BindGroupLayoutEntries, CachedComputePipelineId,
      ComputePipelineDescriptor, PipelineCache, ShaderType,
    },
    renderer::RenderDevice,
    RenderApp,
  },
};
use wgpu::ShaderStages;

use crate::{
  chunk::{Chunk, GpuChunkAttributes, GpuChunkOccupancy, RenderableChunks},
  sun::render::GpuSunLight,
  CHUNK_VOXEL_COUNT,
};

pub struct DirectPassNode {
  query_state: QueryState<(&'static Handle<Chunk>, &'static GlobalTransform)>,
}

impl Node for DirectPassNode {
  fn run<'w>(
    &self,
    _graph: &mut bevy::render::render_graph::RenderGraphContext,
    _render_context: &mut bevy::render::renderer::RenderContext<'w>,
    world: &'w World,
  ) -> Result<(), bevy::render::render_graph::NodeRunError> {
    // let renderable_chunks_list = &world
    //   .get_resource::<RenderableChunks>()
    //   .expect("could not find `RenderableChunks` resource")
    //   .0;

    // // collect hashmap of entity => handle and transform
    // let mut chunks =
    //   EntityHashMap::<(Handle<Chunk>, GlobalTransform)>::default();
    // for entity in renderable_chunks_list.iter() {
    //   let (handle, transform) = self
    //     .query_state
    //     .get_manual(world, *entity)
    //     .expect("failed to find renderable chunk in world");
    //   chunks.insert(*entity, (handle.clone(), *transform));
    // }

    // debug!(
    //   "running `DirectPassNode`, with {} renderable chunks",
    //   chunks.len()
    // );

    Ok(())
  }

  fn update(&mut self, world: &mut World) { self.query_state = world.query(); }
}

impl DirectPassNode {
  pub fn new(world: &mut World) -> Self {
    Self {
      query_state: world.query(),
    }
  }
}

impl FromWorld for DirectPassNode {
  fn from_world(world: &mut World) -> Self { Self::new(world) }
}

#[derive(Debug, ShaderType)]
struct DirectPassUniform {
  current_chunk: u32,
}

#[derive(ShaderType)]
struct DirectPassOutput {
  output: [Vec3; CHUNK_VOXEL_COUNT],
}

impl DirectPassOutput {
  fn new() -> DirectPassOutput {
    DirectPassOutput {
      output: [Vec3::ZERO; CHUNK_VOXEL_COUNT],
    }
  }
}

#[derive(Resource)]
struct DirectPassPipeline {
  bind_group_layout: BindGroupLayout,
  pipeline:          CachedComputePipelineId,
}

impl FromWorld for DirectPassPipeline {
  fn from_world(world: &mut World) -> Self {
    let render_device = world.resource::<RenderDevice>();

    let shader = world
      .resource::<AssetServer>()
      .load("shaders/direct_pass.wgsl");

    let bind_group_layout = render_device.create_bind_group_layout(
      "direct_pass_layout",
      &BindGroupLayoutEntries::with_indices(
        ShaderStages::COMPUTE,
        (
          (0, storage_buffer_read_only::<Vec<GpuChunkOccupancy>>(false)),
          (1, storage_buffer_read_only::<GpuChunkAttributes>(false)),
          (2, uniform_buffer::<Vec<Mat4>>(false)),
          (3, uniform_buffer::<DirectPassUniform>(false)),
          (4, uniform_buffer::<Vec<GpuSunLight>>(false)),
          (5, storage_buffer::<DirectPassOutput>(false)),
        ),
      ),
    );

    let pipeline_cache = world.resource::<PipelineCache>();
    let pipeline =
      pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some(Cow::from("direct_pass_pipeline")),
        layout: vec![bind_group_layout.clone()],
        push_constant_ranges: vec![],
        shader,
        shader_defs: vec![],
        entry_point: Cow::from("update"),
      });

    DirectPassPipeline {
      bind_group_layout,
      pipeline,
    }
  }
}

pub struct DirectPassPlugin;

impl Plugin for DirectPassPlugin {
  fn build(&self, _app: &mut App) {}
  fn finish(&self, app: &mut App) {
    let render_app = app.sub_app_mut(RenderApp);
    render_app.init_resource::<DirectPassPipeline>();
  }
}
