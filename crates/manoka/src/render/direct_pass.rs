use std::{borrow::Cow, num::NonZeroU32};

use bevy::{
  ecs::entity::EntityHashMap,
  prelude::*,
  render::{
    render_asset::RenderAssets,
    render_graph::Node,
    render_resource::{
      binding_types::{
        storage_buffer, storage_buffer_read_only, uniform_buffer,
      },
      BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
      Buffer, BufferDescriptor, CachedComputePipelineId,
      ComputePipelineDescriptor, PipelineCache, ShaderType, StorageBuffer,
    },
    renderer::{RenderDevice, RenderQueue},
    Render, RenderApp, RenderSet,
  },
};
use wgpu::{
  BindGroupLayoutEntry, BindingType, BufferBindingType, BufferUsages,
  ComputePassDescriptor, ShaderStages,
};

use crate::{
  chunk::{Chunk, GpuChunkAttributes, GpuChunkOccupancy},
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
    let bind_groups = world.resource::<DirectPassBindGroups>();

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
    pass.set_bind_group(0, &bind_groups.common, &[]);
    for (_, specific_bind_group) in bind_groups.specific.iter() {
      pass.set_bind_group(1, &specific_bind_group, &[]);
      pass.dispatch_workgroups(16, 16, 16);
    }

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
  entity:                    Entity,
  transform:                 GlobalTransform,
  chunk_asset_id:            AssetId<Chunk>,
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
      entity: entity.clone(),
      transform: transform.clone(),
      chunk_asset_id: chunk_handle.id(),
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
pub struct DirectPassBindGroup(BindGroup);

fn prepare_direct_pass_bind_groups(
  mut commands: Commands,
  pipeline: Res<DirectPassPipeline>,
  renderable_chunks: Res<ChunksToRender>,
  global_buffers: Res<DirectPassGlobalBuffers>,
  sun_light_buffer: Res<SunLightsBuffer>,
  render_device: Res<RenderDevice>,
  chunks: Res<RenderAssets<Chunk>>,
) {
  let common = render_device.create_bind_group(
    Some("direct_pass_common_bind_group"),
    &pipeline.common_bind_group_layout,
    &BindGroupEntries::with_indices((
      (0, global_buffers.om_buffer.binding().unwrap()),
      (1, global_buffers.transform_buffer.binding().unwrap()),
      (2, sun_light_buffer.0.binding().unwrap()),
    )),
  );

  let mut specific = EntityHashMap::default();
  for (entity, renderable_chunk) in renderable_chunks.0.iter() {
    specific.insert(
      entity.clone(),
      render_device.create_bind_group(
        Some("direct_pass_specific_bind_group"),
        &pipeline.specific_bind_group_layout,
        &BindGroupEntries::with_indices((
          (
            0,
            chunks
              .get(renderable_chunk.chunk_asset.id())
              .unwrap()
              .attribute_buffer
              .binding()
              .unwrap(),
          ),
          (1, renderable_chunk.direct_pass_uniform.binding().unwrap()),
          (
            2,
            renderable_chunk
              .direct_pass_output_buffer
              .as_entire_binding(),
          ),
        )),
      ),
    );
  }

  commands.insert_resource(DirectPassBindGroups { common, specific });
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
  common_bind_group_layout:   BindGroupLayout,
  specific_bind_group_layout: BindGroupLayout,
  pipeline:                   CachedComputePipelineId,
}

impl FromWorld for DirectPassPipeline {
  fn from_world(world: &mut World) -> Self {
    let render_device = world.resource::<RenderDevice>();

    let shader = world
      .resource::<AssetServer>()
      .load("shaders/direct_pass.wgsl");

    let bind_group_layout = render_device.create_bind_group_layout(
      "direct_pass_layout",
      // &BindGroupLayoutEntries::with_indices(
      //   ShaderStages::COMPUTE,
      //   (
      //     (0, storage_buffer_read_only::<Vec<GpuChunkOccupancy>>(false)),
      //     (1, storage_buffer_read_only::<Vec<Mat4>>(false)),
      //     (2, storage_buffer_read_only::<Vec<GpuSunLight>>(false)),
      //   ),
      // ),
      &[
        BindGroupLayoutEntry {
          binding:    0,
          visibility: ShaderStages::COMPUTE,
          ty:         BindingType::Buffer {
            ty:                 BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size:   None,
          },
          count:      NonZeroU32::new(MAX_CHUNKS as u32),
        },
        BindGroupLayoutEntry {
          binding:    1,
          visibility: ShaderStages::COMPUTE,
          ty:         BindingType::Buffer {
            ty:                 BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size:   Some(GpuChunkAttributes::min_size()),
          },
          count:      NonZeroU32::new(MAX_CHUNKS as u32),
        },
        BindGroupLayoutEntry {
          binding:    2,
          visibility: ShaderStages::COMPUTE,
          ty:         BindingType::Buffer {
            ty:                 BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size:   None,
          },
          count:      None,
        },
      ],
    );
    let specific_bind_group_layout = render_device.create_bind_group_layout(
      "direct_pass_specifc_layout",
      &BindGroupLayoutEntries::with_indices(
        ShaderStages::COMPUTE,
        (
          (0, storage_buffer_read_only::<GpuChunkAttributes>(false)),
          (1, uniform_buffer::<DirectPassUniform>(false)),
          (2, storage_buffer::<DirectPassOutput>(false)),
        ),
      ),
    );

    let pipeline_cache = world.resource::<PipelineCache>();
    let pipeline =
      pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some(Cow::from("direct_pass_pipeline")),
        layout: vec![
          common_bind_group_layout.clone(),
          specific_bind_group_layout.clone(),
        ],
        push_constant_ranges: vec![],
        shader,
        shader_defs: vec![],
        entry_point: Cow::from("update"),
      });

    DirectPassPipeline {
      common_bind_group_layout,
      specific_bind_group_layout,
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
