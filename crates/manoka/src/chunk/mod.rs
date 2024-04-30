mod inspector;
use bevy::{
  asset::ReflectAsset,
  ecs::system::SystemParamItem,
  prelude::*,
  render::{
    render_asset::{
      PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssetUsages,
    },
    render_resource::{ShaderType, StorageBuffer},
    Extract, Render, RenderApp, RenderSet,
  },
};
use bevy_inspector_egui::inspector_egui_impls::InspectorEguiImpl;

use crate::CHUNK_VOXEL_COUNT;

#[derive(Clone, Debug, Reflect, ShaderType)]
pub struct FullVoxel {
  normal: Vec3,
  color:  Vec3,
}

#[derive(Clone, Debug, Asset, Reflect)]
#[reflect(Asset)]
pub enum Chunk {
  Full { data: Vec<Option<FullVoxel>> },
}

#[derive(Clone, Debug, ShaderType)]
pub struct GpuChunkOccupancy {
  pub occupancy: [u32; CHUNK_VOXEL_COUNT / 32],
}

#[derive(Clone, Debug, ShaderType)]
pub struct GpuChunkAttributes {
  #[size(runtime)]
  attributes: Vec<FullVoxel>,
}

impl Chunk {
  fn into_full(&self) -> Vec<Option<FullVoxel>> {
    match self {
      Self::Full { data } => data.clone(),
    }
  }

  fn prepare_occupancy(&self) -> GpuChunkOccupancy {
    let data = self.into_full();
    let occupancy_map = data.iter().map(|v| v.is_some()).collect::<Vec<_>>();

    // transform array of bools into u32s
    let mut u32_values: [u32; CHUNK_VOXEL_COUNT / 32] =
      [0; CHUNK_VOXEL_COUNT / 32];
    for (i, chunk) in occupancy_map.chunks(32).enumerate() {
      let mut u32_value = 0;
      for (j, &bit) in chunk.iter().enumerate() {
        if bit {
          u32_value |= 1 << j;
        }
      }
      u32_values[i] = u32_value;
    }

    GpuChunkOccupancy {
      occupancy: u32_values,
    }
  }

  fn prepare_attributes(&self) -> GpuChunkAttributes {
    let data = self.into_full();
    GpuChunkAttributes {
      attributes: data.into_iter().flatten().collect(),
    }
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

  #[allow(dead_code)]
  pub fn new_empty() -> Self {
    Self::Full {
      data: vec![None; CHUNK_VOXEL_COUNT],
    }
  }
}

impl RenderAsset for Chunk {
  type PreparedAsset = GpuChunk;

  type Param = ();

  fn asset_usage(&self) -> RenderAssetUsages {
    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
  }

  fn prepare_asset(
    self,
    _param: &mut SystemParamItem<Self::Param>,
  ) -> Result<Self::PreparedAsset, PrepareAssetError<Self>> {
    debug!("creating `GpuChunk`");
    Ok(GpuChunk {
      _occupancy:  self.prepare_occupancy(),
      _attributes: self.prepare_attributes(),
    })
  }
}

pub struct GpuChunk {
  _occupancy:  GpuChunkOccupancy,
  _attributes: GpuChunkAttributes,
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
