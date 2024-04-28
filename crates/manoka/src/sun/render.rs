use bevy::{
  prelude::*,
  render::{Extract, RenderApp},
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
  sun_lights: Extract<Query<(Entity, &SunLight, &GlobalTransform)>>,
) {
  for (entity, sun_light, transform) in sun_lights.iter() {
    commands.get_or_spawn(entity).insert(ExtractedSunLight {
      color:       sun_light.color.as_linear_rgba_f32(),
      illuminance: sun_light.illuminance,
      transform:   *transform,
    });
  }
}

impl Plugin for SunRenderPlugin {
  fn build(&self, app: &mut App) {
    let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
      panic!("render_app not found");
    };

    render_app.add_systems(ExtractSchedule, extract_sun_lights);
  }
}
