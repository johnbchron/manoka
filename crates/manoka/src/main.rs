mod chunk;

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
  winit::WinitPlugin,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use self::chunk::Chunk;

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
    WindowPlugin::default(),
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
    .add_plugins((ManokaRenderPlugin, crate::chunk::ChunkPlugin))
    .add_systems(Startup, setup);

  bevy_mod_debugdump::print_render_graph(&mut app);

  app.run();
}

fn setup(mut commands: Commands, mut chunks: ResMut<Assets<Chunk>>) {
  let chunk_handle = chunks.add(Chunk::debug_red_sphere_chunk());

  commands.spawn((
    chunk_handle,
    SpatialBundle::default(),
    Name::new("test_chunk"),
  ));
}

pub struct ManokaRenderPlugin;

impl Plugin for ManokaRenderPlugin {
  fn build(&self, app: &mut App) {
    let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
      panic!("render_app not found");
    };

    let mut graph = render_app.world.resource_mut::<RenderGraph>();
    graph.add_node(CustomRenderNodeLabel, CustomRenderNode);
    graph.add_node_edge(CustomRenderNodeLabel, CameraDriverLabel);
  }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, RenderLabel)]
struct CustomRenderNodeLabel;

struct CustomRenderNode;
impl Node for CustomRenderNode {
  fn run<'w>(
    &self,
    _graph: &mut RenderGraphContext,
    render_context: &mut RenderContext<'w>,
    world: &'w World,
  ) -> Result<(), NodeRunError> {
    let Some((_e, window)) = world.resource::<ExtractedWindows>().iter().next()
    else {
      error!("no window found");
      return Ok(());
    };

    let Some(texture_view) = window.swap_chain_texture_view.as_ref() else {
      error!("no swap chain texture view");
      return Ok(());
    };

    let command_encoder = render_context.command_encoder();

    // Clear the screen
    let _render_pass =
      command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label:                    Some("Custom Clear Render Pass"),
        color_attachments:        &[Some(wgpu::RenderPassColorAttachment {
          view:           texture_view,
          resolve_target: None,
          ops:            wgpu::Operations {
            load:  wgpu::LoadOp::Clear(wgpu::Color {
              r: 0.1,
              g: 0.2,
              b: 0.3,
              a: 1.0,
            }),
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set:      None,
        timestamp_writes:         None,
      });

    Ok(())
  }
}
