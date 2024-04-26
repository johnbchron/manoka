mod chunk;
mod render;
mod sun;

use std::f32::consts::PI;

use bevy::{
  diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
  prelude::*,
  window::PresentMode,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use sun::SunPlugin;

use self::{chunk::Chunk, render::ManokaRenderPlugin, sun::SunLight};
use crate::chunk::ChunkPlugin;

pub const CHUNK_VOXEL_COUNT: usize = 64 * 64 * 64;

fn main() {
  let mut app = App::new();

  // main first-party plugins
  app.add_plugins(DefaultPlugins.set(WindowPlugin {
    primary_window: Some(Window {
      title: "manoka".to_string(),
      mode: bevy::window::WindowMode::BorderlessFullscreen,
      present_mode: PresentMode::AutoNoVsync,
      ..default()
    }),
    ..default()
  }));

  // other first-party plugins
  app.add_plugins((
    LogDiagnosticsPlugin::default(),
    FrameTimeDiagnosticsPlugin::default(),
  ));

  // third party plugins
  app.add_plugins(WorldInspectorPlugin::default());

  // first party logic
  app
    .add_plugins((ManokaRenderPlugin, ChunkPlugin, SunPlugin))
    .add_systems(Startup, setup);

  bevy_mod_debugdump::print_render_graph(&mut app);

  app.run();
}

fn setup(mut commands: Commands, mut chunks: ResMut<Assets<Chunk>>) {
  let chunk_handle = chunks.add(Chunk::debug_red_sphere_chunk());

  // spawn a chunk
  commands.spawn((
    chunk_handle,
    SpatialBundle::default(),
    Name::new("test_chunk"),
  ));

  // spawn a sun
  commands.spawn((
    SunLight {
      color:       Color::WHITE,
      illuminance: 1000.0,
    },
    SpatialBundle::from_transform(Transform::from_rotation(
      Quat::from_axis_angle(Vec3::X, PI / 2.0),
    )),
    Name::new("sun"),
  ));

  // spawn a camera
  commands.spawn(Camera3dBundle {
    camera: Camera {
      hdr: true,
      ..default()
    },

    transform: Transform::from_xyz(-5.0, 50.0_f32.sqrt(), 5.0),
    ..default()
  });
}
