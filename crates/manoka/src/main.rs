mod chunk;
mod render;
mod sun;

use std::f32::consts::PI;

use bevy::{
  a11y::AccessibilityPlugin,
  diagnostic::DiagnosticsPlugin,
  input::InputPlugin,
  log::LogPlugin,
  prelude::*,
  render::{
    graph::CameraDriverLabel,
    render_graph::{
      Node, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel,
    },
    renderer::RenderContext,
    view::ExtractedWindows,
    RenderApp, RenderPlugin,
  },
  scene::ScenePlugin,
  window::PresentMode,
  winit::WinitPlugin,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use sun::SunPlugin;

use self::{chunk::Chunk, render::ManokaRenderPlugin, sun::SunLight};
use crate::chunk::ChunkPlugin;

pub const CHUNK_VOXEL_COUNT: usize = 64 * 64 * 64;

fn main() {
  let mut app = App::new();

  // modified `DefaultPlugins`
  app.add_plugins((
    MinimalPlugins,
    LogPlugin::default(),
    HierarchyPlugin,
    DiagnosticsPlugin::default(),
    InputPlugin::default(),
    WindowPlugin {
      primary_window: Some(Window {
        title: "manoka".to_string(),
        mode: bevy::window::WindowMode::BorderlessFullscreen,
        present_mode: PresentMode::AutoNoVsync,
        ..default()
      }),
      ..default()
    },
    AccessibilityPlugin,
    AssetPlugin::default(),
    ScenePlugin::default(),
    WinitPlugin::default(),
    RenderPlugin::default(),
    ImagePlugin::default(),
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
}
