mod inspector;

use bevy::{asset::ReflectAsset, prelude::*};
use bevy_inspector_egui::inspector_egui_impls::InspectorEguiImpl;

#[derive(Clone, Debug, Reflect)]
pub struct FullVoxel {
  normal: Vec3,
  color:  Vec3,
}

#[derive(Clone, Debug, Asset, Reflect)]
#[reflect(Asset)]
pub enum Chunk {
  Full { data: Vec<Option<FullVoxel>> },
}

impl Chunk {
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
}

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
  fn build(&self, app: &mut App) {
    app
      .register_asset_reflect::<Chunk>()
      .init_asset::<Chunk>()
      .register_type_data::<Chunk, InspectorEguiImpl>();
  }
}
