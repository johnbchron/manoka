use bevy::{
  prelude::*,
  render::{
    render_graph::{EmptyNode, RenderGraphApp, RenderLabel, RenderSubGraph},
    RenderApp,
  },
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
pub struct CoreVoxel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub enum NodeVoxel {
  DirectPass,
}

pub struct ManokaRenderPlugin;

impl Plugin for ManokaRenderPlugin {
  fn build(&self, app: &mut App) {
    let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
      panic!("render_app not found");
    };

    render_app.add_render_sub_graph(CoreVoxel);
    render_app
      .add_render_graph_node::<EmptyNode>(CoreVoxel, NodeVoxel::DirectPass);
  }
}

pub struct DirectPassNode;
