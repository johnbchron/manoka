use std::{borrow::Cow, num::NonZeroU32};

use bevy::{
  prelude::*,
  render::{
    render_asset::RenderAssets,
    render_graph::Node,
    render_resource::{
      binding_types::{storage_buffer_read_only, storage_buffer_sized},
      BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
      BindingResource, Buffer, BufferDescriptor, CachedComputePipelineId,
      ComputePipelineDescriptor, PipelineCache, ShaderType, StorageBuffer,
    },
    renderer::{RenderDevice, RenderQueue},
    Render, RenderApp, RenderSet,
  },
};
use wgpu::{BufferUsages, ComputePassDescriptor, ShaderStages};

use crate::{
  chunk::Chunk,
  sun::render::{GpuSunLight, SunLightsBuffer},
  CHUNK_VOXEL_COUNT, MAX_CHUNKS,
};

pub struct DirectPassNode {
  query_state: QueryState<(&'static Handle<Chunk>, &'static GlobalTransform)>,
}

impl Node for DirectPassNode {
  fn run<'w>(
    &self,
    _graph: &mut bevy::render::render_graph::RenderGraphContext,
    render_context: &mut bevy::render::renderer::RenderContext<'w>,
    world: &'w World,
  ) -> Result<(), bevy::render::render_graph::NodeRunError> {
    let pipelines = world.resource::<DirectPassPipeline>();
    let pipeline_cache = world.resource::<PipelineCache>();
    let bind_group = world.resource::<DirectPassBindGroup>();

    let Some(pipeline) =
      pipeline_cache.get_compute_pipeline(pipelines.pipeline)
    else {
      return Ok(());
    };

    render_context
      .command_encoder()
      .push_debug_group("direct_pass");

    let mut pass = render_context.command_encoder().begin_compute_pass(
      &ComputePassDescriptor {
        label:            Some("direct_pass_main_pass"),
        timestamp_writes: None,
      },
    );
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &bind_group.0, &[]);
    pass.dispatch_workgroups(16, 16, (16 * bind_group.1) as _);

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

#[derive()]
pub struct RenderedChunk {
  _entity:                   Entity,
  transform:                 GlobalTransform,
  _chunk_asset_id:           AssetId<Chunk>,
  occupancy_buffer:          Buffer,
  attribute_buffer:          Buffer,
  direct_pass_output_buffer: Buffer,
}

#[derive(Resource)]
pub struct DirectPassGlobalBuffers {
  transform_buffer: StorageBuffer<Vec<Mat4>>,
}

#[derive(Resource)]
pub struct ChunksToRender(pub Vec<RenderedChunk>);

#[allow(clippy::type_complexity)]
fn prepare_renderable_chunks(
  mut commands: Commands,
  query: Query<(Entity, &Handle<Chunk>, &GlobalTransform, &ViewVisibility)>,
  render_device: Res<RenderDevice>,
  render_queue: Res<RenderQueue>,
  chunks: Res<RenderAssets<Chunk>>,
) {
  // get everything from the query, filter it by only view-visible items, then
  // sort by the entity index.
  let mut extracted_chunks = query
    .iter()
    .filter(|(_, _, _, vv)| vv.get())
    .collect::<Vec<_>>();
  extracted_chunks.sort_unstable_by_key(|(e, _, _, _)| e.index());

  // collect the `RenderedChunk`s from the extracted chunks
  let mut chunks_to_render = Vec::new();
  for (entity, chunk_handle, transform, _) in extracted_chunks.into_iter() {
    // create the output buffer
    let direct_pass_output_buffer =
      render_device.create_buffer(&BufferDescriptor {
        label:              Some("direct_pass_output_buffer"),
        size:               DirectPassOutput::min_size().get(),
        usage:              BufferUsages::STORAGE,
        mapped_at_creation: false,
      });

    // pull the occupancy and attribute buffers out of the chunk asset
    let occupancy_buffer = chunks
      .get(chunk_handle.id())
      .unwrap()
      .occupancy_buffer
      .buffer()
      .unwrap()
      .clone();
    let attribute_buffer = chunks
      .get(chunk_handle.id())
      .unwrap()
      .attribute_buffer
      .buffer()
      .unwrap()
      .clone();

    chunks_to_render.push(RenderedChunk {
      _entity: entity.clone(),
      transform: transform.clone(),
      _chunk_asset_id: chunk_handle.id(),
      occupancy_buffer,
      attribute_buffer,
      direct_pass_output_buffer,
    })
  }

  let mut global_transform_buffer = StorageBuffer::from(
    chunks_to_render
      .iter()
      .map(|e| e.transform.compute_matrix())
      .collect::<Vec<_>>(),
  );
  global_transform_buffer.write_buffer(&render_device, &render_queue);

  commands.insert_resource(ChunksToRender(chunks_to_render));
  commands.insert_resource(DirectPassGlobalBuffers {
    transform_buffer: global_transform_buffer,
  });
}

#[derive(Resource)]
/// This contains the bind group and the chunk count.
pub struct DirectPassBindGroup(BindGroup, usize);

fn prepare_direct_pass_bind_groups(
  mut commands: Commands,
  pipeline: Res<DirectPassPipeline>,
  chunks_to_render: Res<ChunksToRender>,
  global_buffers: Res<DirectPassGlobalBuffers>,
  sun_light_buffer: Res<SunLightsBuffer>,
  render_device: Res<RenderDevice>,
) {
  let om_buffers = chunks_to_render
    .0
    .iter()
    .map(|c| c.occupancy_buffer.as_entire_buffer_binding())
    .collect::<Vec<_>>();
  let attribute_buffers = chunks_to_render
    .0
    .iter()
    .map(|c| c.attribute_buffer.as_entire_buffer_binding())
    .collect::<Vec<_>>();
  let output_buffers = chunks_to_render
    .0
    .iter()
    .map(|c| c.direct_pass_output_buffer.as_entire_buffer_binding())
    .collect::<Vec<_>>();

  let bind_group = render_device.create_bind_group(
    Some("direct_pass_common_bind_group"),
    &pipeline.bind_group_layout,
    &BindGroupEntries::with_indices((
      (0, BindingResource::BufferArray(&om_buffers)),
      (1, BindingResource::BufferArray(&attribute_buffers)),
      (2, BindingResource::BufferArray(&output_buffers)),
      (3, global_buffers.transform_buffer.binding().unwrap()),
      (4, sun_light_buffer.0.binding().unwrap()),
    )),
  );

  commands
    .insert_resource(DirectPassBindGroup(bind_group, chunks_to_render.0.len()));
}

#[derive(Debug, ShaderType)]
struct DirectPassUniform {
  current_chunk: u32,
}

#[derive(ShaderType)]
struct DirectPassOutput {
  output: [Vec3; CHUNK_VOXEL_COUNT],
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
          // all OM buffer bindings
          (
            0,
            storage_buffer_sized(false, None)
              .count(NonZeroU32::new(MAX_CHUNKS as _).unwrap()),
          ),
          // all attribute buffer bindings
          (
            1,
            storage_buffer_sized(false, None)
              .count(NonZeroU32::new(MAX_CHUNKS as _).unwrap()),
          ),
          // all output buffers
          (
            2,
            storage_buffer_sized(false, None)
              .count(NonZeroU32::new(MAX_CHUNKS as _).unwrap()),
          ),
          // chunk transforms
          (3, storage_buffer_read_only::<Vec<Mat4>>(false)),
          // sun parameters
          (4, storage_buffer_read_only::<Vec<GpuSunLight>>(false)),
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
    render_app.add_systems(
      Render,
      (
        prepare_renderable_chunks.in_set(RenderSet::Prepare),
        prepare_direct_pass_bind_groups.in_set(RenderSet::PrepareBindGroups),
      ),
    );
  }
}
