mod direct_pass;

use bevy::{
  prelude::*,
  render::{
    render_graph::{RenderGraphApp, RenderLabel, RenderSubGraph},
    RenderApp,
  },
};

use self::direct_pass::{DirectPassNode, DirectPassPlugin};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
pub struct CoreVoxel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub enum NodeVoxel {
  DirectPass,
}

pub struct ManokaRenderPlugin;

impl Plugin for ManokaRenderPlugin {
  fn build(&self, app: &mut App) {
    app.add_plugins(DirectPassPlugin);

    let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
      panic!("render_app not found");
    };

    render_app.add_render_sub_graph(CoreVoxel);
    render_app.add_render_graph_node::<DirectPassNode>(
      CoreVoxel,
      NodeVoxel::DirectPass,
    );
  }
}
