mod render;

use bevy::prelude::*;

use self::render::SunRenderPlugin;

#[derive(Clone, Debug, Component, Reflect)]
#[reflect(Component)]
pub struct SunLight {
  pub color:       Color,
  pub illuminance: f32,
}

pub struct SunPlugin;

impl Plugin for SunPlugin {
  fn build(&self, app: &mut App) {
    app.register_type::<SunLight>().add_plugins(SunRenderPlugin);
  }
}
