use bevy::{
  prelude::*,
  render::{
    render_resource::{ShaderType, StorageBuffer},
    renderer::{RenderDevice, RenderQueue},
    Extract, Render, RenderApp, RenderSet,
  },
};

use super::SunLight;

pub struct SunRenderPlugin;

#[derive(Clone, Debug, Component)]
pub struct ExtractedSunLight {
  /// This is linear RGBA
  color:       [f32; 4],
  illuminance: f32,
  transform:   GlobalTransform,
}

fn extract_sun_lights(
  mut commands: Commands,
  query: Extract<Query<(Entity, &SunLight, &GlobalTransform)>>,
) {
  for (entity, sun_light, transform) in query.iter() {
    commands.get_or_spawn(entity).insert(ExtractedSunLight {
      color:       sun_light.color.as_linear_rgba_f32(),
      illuminance: sun_light.illuminance,
      transform:   *transform,
    });
  }
}

#[derive(Clone, Debug, ShaderType)]
pub struct GpuSunLight {
  color:       [f32; 4],
  illuminance: f32,
  direction:   Vec3,
}

impl From<ExtractedSunLight> for GpuSunLight {
  fn from(value: ExtractedSunLight) -> Self {
    GpuSunLight {
      color:       value.color,
      illuminance: value.illuminance,
      direction:   value.transform.forward(),
    }
  }
}

#[derive(Resource)]
pub struct SunLightsBuffer(pub StorageBuffer<Vec<GpuSunLight>>);

fn prepare_sun_lights(
  mut commands: Commands,
  query: Query<&ExtractedSunLight>,
  render_device: Res<RenderDevice>,
  render_queue: Res<RenderQueue>,
) {
  let mut sun_lights_buffer = StorageBuffer::from(
    query
      .iter()
      .cloned()
      .map(GpuSunLight::from)
      .collect::<Vec<_>>(),
  );
  sun_lights_buffer.write_buffer(&render_device, &render_queue);

  commands.insert_resource(SunLightsBuffer(sun_lights_buffer));
}

impl Plugin for SunRenderPlugin {
  fn build(&self, app: &mut App) {
    let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
      panic!("render_app not found");
    };

    render_app
      .add_systems(ExtractSchedule, extract_sun_lights)
      .add_systems(Render, prepare_sun_lights.in_set(RenderSet::Prepare));
  }
}
