mod inspector;
use bevy::{
  asset::ReflectAsset,
  ecs::system::{lifetimeless::SRes, SystemParamItem},
  prelude::*,
  render::{
    render_asset::{
      PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssetUsages,
    },
    render_resource::{Buffer, ShaderType},
    renderer::RenderDevice,
    Extract, Render, RenderApp, RenderSet,
  },
};
use bevy_inspector_egui::inspector_egui_impls::InspectorEguiImpl;
use wgpu::{util::BufferInitDescriptor, BufferUsages};
use zerocopy::AsBytes;

use crate::CHUNK_VOXEL_COUNT;

#[derive(Clone, Debug, Reflect, ShaderType)]
pub struct FullVoxel {
  normal: Vec3,
  color:  Vec3,
}

impl FullVoxel {
  pub fn get_bytes(&self) -> Vec<u8> {
    let mut data: Vec<u8> = vec![0; Self::min_size().get() as _];
    data[0..4].copy_from_slice(&self.normal.x.to_le_bytes());
    data[4..8].copy_from_slice(&self.normal.y.to_le_bytes());
    data[8..12].copy_from_slice(&self.normal.z.to_le_bytes());
    data[12..16].copy_from_slice(&self.color.x.to_le_bytes());
    data[16..20].copy_from_slice(&self.color.y.to_le_bytes());
    data[20..24].copy_from_slice(&self.color.z.to_le_bytes());
    data
  }
}

#[derive(Clone, Debug, Asset, Reflect)]
#[reflect(Asset)]
pub enum Chunk {
  Full { data: Vec<Option<FullVoxel>> },
}

impl Chunk {
  pub fn prepare_buffer_full_data(&self) -> Vec<u8> {
    let data = match self {
      Self::Full { data } => data,
    };

    FullVoxel::assert_uniform_compat();

    let mut final_bytes: Vec<u8> = vec![];

    // build the occupancy map
    let occupancy_map = data.iter().map(|v| v.is_some()).collect::<Vec<_>>();
    final_bytes.append(&mut occupancy_map.as_bytes().to_vec());

    // collect the attributes into a dense list
    let attribute_size = FullVoxel::min_size().get() as usize;
    let dense_attributes =
      data.clone().into_iter().flatten().collect::<Vec<_>>();
    let attribute_count = dense_attributes.len();

    // copy the attribute bytes into a vector
    let mut attribute_offset = 0;
    let mut attribute_bytes: Vec<u8> =
      vec![0; attribute_size * attribute_count];
    for attribute in dense_attributes {
      attribute_bytes[attribute_offset..attribute_offset + attribute_size]
        .copy_from_slice(&attribute.get_bytes());
      attribute_offset += attribute_size;
    }
    final_bytes.append(&mut attribute_bytes);

    debug!(
      "created a chunk full_data buffer with {} bytes",
      final_bytes.len()
    );
    final_bytes
  }

  pub fn debug_red_sphere_chunk() -> Self {
    let mut buffer = Vec::with_capacity(crate::CHUNK_VOXEL_COUNT);

    for z in 0..64 {
      for y in 0..64 {
        for x in 0..64 {
          let pos = IVec3::new(x, y, z);
          let frac_pos =
            Vec3::new(pos.x as _, pos.y as _, pos.z as _) / 32.0 - 1.0;
          let occupied = frac_pos.length() <= 1.0;
          let normal = frac_pos.normalize();
          let color = normal / 2.0 + 1.0;

          buffer.push(occupied.then_some(FullVoxel { normal, color }));
        }
      }
    }

    Chunk::Full { data: buffer }
  }

  pub fn prepare_buffer_occupancy(&self) -> Vec<u8> {
    let data = match self {
      Self::Full { data } => data,
    };

    FullVoxel::assert_uniform_compat();

    let mut final_bytes: Vec<u8> = vec![];

    // build the occupancy map
    let occupancy_map = data.iter().map(|v| v.is_some()).collect::<Vec<_>>();
    final_bytes.append(&mut occupancy_map.as_bytes().to_vec());

    debug!(
      "created a chunk occupancy buffer with {} bytes",
      final_bytes.len()
    );
    final_bytes
  }

  #[allow(dead_code)]
  pub fn new_empty() -> Self {
    Self::Full {
      data: vec![None; CHUNK_VOXEL_COUNT],
    }
  }
}

impl RenderAsset for Chunk {
  type PreparedAsset = GpuChunk;

  type Param = SRes<RenderDevice>;

  fn asset_usage(&self) -> RenderAssetUsages {
    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
  }

  fn prepare_asset(
    self,
    param: &mut SystemParamItem<Self::Param>,
  ) -> Result<Self::PreparedAsset, PrepareAssetError<Self>> {
    let full_data_buffer = self.prepare_buffer_full_data();
    let occupancy_buffer = self.prepare_buffer_occupancy();
    Ok(GpuChunk {
      _occupancy_buffer: param.create_buffer_with_data(&BufferInitDescriptor {
        label:    Some("chunk_occupancy"),
        contents: &occupancy_buffer,
        usage:    BufferUsages::COPY_SRC,
      }),
      _full_data_buffer: param.create_buffer_with_data(&BufferInitDescriptor {
        label:    Some("chunk_full_data"),
        usage:    BufferUsages::COPY_SRC,
        contents: &full_data_buffer,
      }),
    })
  }
}

pub struct GpuChunk {
  _occupancy_buffer: Buffer,
  _full_data_buffer: Buffer,
}

#[allow(clippy::type_complexity)]
fn extract_chunk_entities(
  mut commands: Commands,
  query: Extract<
    Query<(Entity, &Handle<Chunk>, &GlobalTransform, &ViewVisibility)>,
  >,
) {
  for (entity, chunk_handle, transform, visibility) in query.iter() {
    commands.get_or_spawn(entity).insert((
      chunk_handle.clone(),
      *transform,
      *visibility,
    ));
  }
}

#[derive(Resource)]
pub struct RenderableChunks(pub Vec<Entity>);

#[allow(clippy::type_complexity)]
fn prepare_renderable_chunks(
  mut commands: Commands,
  query: Query<
    (Entity, &ViewVisibility),
    (With<Handle<Chunk>>, With<GlobalTransform>),
  >,
) {
  commands.insert_resource(RenderableChunks(
    query
      .iter()
      .filter(|(_, vv)| vv.get())
      .map(|(e, _)| e)
      .collect(),
  ));
}

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
  fn build(&self, app: &mut App) {
    app
      .register_asset_reflect::<Chunk>()
      .init_asset::<Chunk>()
      .add_plugins(RenderAssetPlugin::<Chunk>::default())
      .register_type_data::<Chunk, InspectorEguiImpl>();
  }

  fn finish(&self, app: &mut App) {
    let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
      panic!("render_app not found");
    };

    render_app
      .add_systems(ExtractSchedule, extract_chunk_entities)
      .add_systems(
        Render,
        prepare_renderable_chunks.in_set(RenderSet::Prepare),
      );
  }
}
