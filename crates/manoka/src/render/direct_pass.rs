use bevy::{
  ecs::entity::EntityHashMap, prelude::*, render::render_graph::Node,
};

use crate::chunk::{Chunk, RenderableChunks};

pub struct DirectPassNode {
  query_state: QueryState<(&'static Handle<Chunk>, &'static GlobalTransform)>,
}

impl Node for DirectPassNode {
  fn run<'w>(
    &self,
    _graph: &mut bevy::render::render_graph::RenderGraphContext,
    _render_context: &mut bevy::render::renderer::RenderContext<'w>,
    world: &'w World,
  ) -> Result<(), bevy::render::render_graph::NodeRunError> {
    // let renderable_chunks_list = &world
    //   .get_resource::<RenderableChunks>()
    //   .expect("could not find `RenderableChunks` resource")
    //   .0;

    // // collect hashmap of entity => handle and transform
    // let mut chunks =
    //   EntityHashMap::<(Handle<Chunk>, GlobalTransform)>::default();
    // for entity in renderable_chunks_list.iter() {
    //   let (handle, transform) = self
    //     .query_state
    //     .get_manual(world, *entity)
    //     .expect("failed to find renderable chunk in world");
    //   chunks.insert(*entity, (handle.clone(), *transform));
    // }

    // debug!(
    //   "running `DirectPassNode`, with {} renderable chunks",
    //   chunks.len()
    // );

    Ok(())
  }

  fn update(&mut self, world: &mut World) { self.query_state = world.query(); }
}

impl DirectPassNode {
  pub fn new(world: &mut World) -> Self {
    Self {
      query_state: world.query(),
    }
  }
}

impl FromWorld for DirectPassNode {
  fn from_world(world: &mut World) -> Self { Self::new(world) }
}

pub struct DirectPassPlugin;

impl Plugin for DirectPassPlugin {
  fn build(&self, _app: &mut App) {}
}
