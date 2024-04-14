//! A compute shader that simulates Conway's Game of Life.
//!
//! Compute shaders use the GPU for computing arbitrary information, that may be
//! independent of what is rendered to the screen.

use std::borrow::Cow;

use bevy::{
  prelude::*,
  render::{
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_asset::{RenderAssetUsages, RenderAssets},
    render_graph::{self, RenderGraph, RenderLabel},
    render_resource::*,
    renderer::{RenderContext, RenderDevice},
    texture::FallbackImage,
    Render, RenderApp, RenderSet,
  },
};

const WINDOW_SIZE: (u32, u32) = (1280, 720);
const WORKGROUP_SIZE: u32 = 32;
const CHUNK_SIZE: usize = 64 * 64 * 64;

fn main() {
  App::new()
    .insert_resource(ClearColor(Color::BLACK))
    .add_plugins((
      DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
          // uncomment for unthrottled FPS
          present_mode: bevy::window::PresentMode::AutoNoVsync,
          ..default()
        }),
        ..default()
      }),
      RaytraceComputePlugin,
    ))
    .add_systems(Startup, setup)
    .add_systems(Update, move_camera)
    .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
  let mut image = Image::new_fill(
    Extent3d {
      width:                 WINDOW_SIZE.0,
      height:                WINDOW_SIZE.1,
      depth_or_array_layers: 1,
    },
    TextureDimension::D2,
    &[0, 0, 0, 255],
    TextureFormat::Rgba8Unorm,
    RenderAssetUsages::RENDER_WORLD,
  );
  image.texture_descriptor.usage = TextureUsages::COPY_DST
    | TextureUsages::STORAGE_BINDING
    | TextureUsages::TEXTURE_BINDING;
  let image = images.add(image);

  commands.spawn(SpriteBundle {
    sprite: Sprite {
      custom_size: Some(Vec2::new(WINDOW_SIZE.0 as f32, WINDOW_SIZE.1 as f32)),
      ..default()
    },
    texture: image.clone(),
    ..default()
  });
  commands.spawn(Camera2dBundle::default());

  let mut voxel_buffer: Vec<Voxel> = Vec::with_capacity(CHUNK_SIZE);

  for z in 0..64 {
    for y in 0..64 {
      for x in 0..64 {
        let pos = Vec3::new(x as f32, y as f32, z as f32) / 64.0 - 0.5;
        let material = (pos.length() < 0.25) as u32;
        let normal = pos.normalize();
        voxel_buffer.push(Voxel { material, normal });
      }
    }
  }

  commands.insert_resource(RaytraceInputs {
    camera:         Camera {
      pos:     Vec3::new(0.0, 40.0, 40.0),
      look_at: Vec3::ZERO,
      fov:     60.0_f32.to_radians(),
      up:      Vec3::Y,
    },
    chunk_data:     voxel_buffer,
    output_texture: image,
  });
}

fn move_camera(
  keys: Res<ButtonInput<KeyCode>>,
  time: Res<Time>,
  mut params: ResMut<RaytraceInputs>,
) {
  if keys.pressed(KeyCode::KeyW) {
    params.camera.pos.z -= time.delta().as_secs_f32() * 5.0;
  }
  if keys.pressed(KeyCode::KeyA) {
    params.camera.pos.x -= time.delta().as_secs_f32() * 5.0;
  }
  if keys.pressed(KeyCode::KeyS) {
    params.camera.pos.z += time.delta().as_secs_f32() * 5.0;
  }
  if keys.pressed(KeyCode::KeyD) {
    params.camera.pos.x += time.delta().as_secs_f32() * 5.0;
  }
  if keys.pressed(KeyCode::Space) {
    params.camera.pos.y += time.delta().as_secs_f32() * 5.0;
  }
  if keys.pressed(KeyCode::ShiftLeft) {
    params.camera.pos.y -= time.delta().as_secs_f32() * 5.0;
  }
}

struct RaytraceComputePlugin;

/// The label is the unique instance of the node in the node graph.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct RaytraceLabel;

impl Plugin for RaytraceComputePlugin {
  fn build(&self, app: &mut App) {
    // Extract the game of life image resource from the main world into the
    // render world for operation on by the compute shader and display on
    // the sprite.
    app.add_plugins(ExtractResourcePlugin::<RaytraceInputs>::default());
    let render_app = app.sub_app_mut(RenderApp);
    render_app.add_systems(
      Render,
      prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
    );

    let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
    render_graph.add_node(RaytraceLabel, RaytraceNode::default());
    render_graph
      .add_node_edge(RaytraceLabel, bevy::render::graph::CameraDriverLabel);
  }

  fn finish(&self, app: &mut App) {
    let render_app = app.sub_app_mut(RenderApp);
    render_app.init_resource::<RaytracePipeline>();
  }
}

#[derive(ShaderType, Clone)]
pub struct Voxel {
  pub material: u32,
  pub normal:   Vec3,
}

#[derive(ShaderType, Clone)]
pub struct Camera {
  pos:     Vec3,
  look_at: Vec3,
  fov:     f32,
  up:      Vec3,
}

/// A resource containing the inputs that will be eventually used in the shader.
#[derive(Resource, Clone, ExtractResource, AsBindGroup)]
struct RaytraceInputs {
  #[uniform(0)]
  camera:         Camera,
  #[storage(1, read_only, visibility(compute))]
  chunk_data:     Vec<Voxel>,
  #[storage_texture(2, image_format = Rgba8Unorm, access = ReadWrite)]
  output_texture: Handle<Image>,
}

/// The bind group is the actual collection of resources to be bound in a
/// shader. Its format is determined by its BindGroupLayout.
#[derive(Resource)]
struct RaytraceInputsBindGroup(BindGroup);

fn prepare_bind_group(
  mut commands: Commands,
  pipeline: Res<RaytracePipeline>,
  gpu_images: Res<RenderAssets<Image>>,
  fallback_image: Res<FallbackImage>,
  inputs: Res<RaytraceInputs>,
  render_device: Res<RenderDevice>,
) {
  let bind_group = inputs.as_bind_group(
    &pipeline.texture_bind_group_layout,
    &render_device,
    &gpu_images,
    &fallback_image,
  );
  commands
    .insert_resource(RaytraceInputsBindGroup(bind_group.unwrap().bind_group));
}

/// The pipeline is the aggregate descriptor of data layout and instructions to
/// the GPU.
#[derive(Resource)]
struct RaytracePipeline {
  texture_bind_group_layout: BindGroupLayout,
  update_pipeline:           CachedComputePipelineId,
}

impl FromWorld for RaytracePipeline {
  fn from_world(world: &mut World) -> Self {
    let render_device = world.resource::<RenderDevice>();
    let texture_bind_group_layout =
      RaytraceInputs::bind_group_layout(render_device);
    let shader = world
      .resource::<AssetServer>()
      .load("shaders/raytrace.wgsl");
    let pipeline_cache = world.resource::<PipelineCache>();
    let update_pipeline =
      pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: None,
        layout: vec![texture_bind_group_layout.clone()],
        push_constant_ranges: Vec::new(),
        shader,
        shader_defs: vec![],
        entry_point: Cow::from("update"),
      });

    RaytracePipeline {
      texture_bind_group_layout,
      update_pipeline,
    }
  }
}

enum RaytraceState {
  Loading,
  Update,
}

struct RaytraceNode {
  state: RaytraceState,
}

impl Default for RaytraceNode {
  fn default() -> Self {
    Self {
      state: RaytraceState::Loading,
    }
  }
}

impl render_graph::Node for RaytraceNode {
  fn update(&mut self, world: &mut World) {
    let pipeline = world.resource::<RaytracePipeline>();
    let pipeline_cache = world.resource::<PipelineCache>();

    // if the corresponding pipeline has loaded, transition to the next stage
    match self.state {
      RaytraceState::Loading => {
        if let CachedPipelineState::Ok(_) =
          pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
        {
          self.state = RaytraceState::Update;
        }
      }
      RaytraceState::Update => {}
    }
  }

  fn run(
    &self,
    _graph: &mut render_graph::RenderGraphContext,
    render_context: &mut RenderContext,
    world: &World,
  ) -> Result<(), render_graph::NodeRunError> {
    let texture_bind_group = &world.resource::<RaytraceInputsBindGroup>().0;
    let pipeline_cache = world.resource::<PipelineCache>();
    let pipeline = world.resource::<RaytracePipeline>();

    let mut pass = render_context
      .command_encoder()
      .begin_compute_pass(&ComputePassDescriptor::default());

    pass.set_bind_group(0, texture_bind_group, &[]);

    // select the pipeline based on the current state
    match self.state {
      RaytraceState::Loading => {}
      RaytraceState::Update => {
        let update_pipeline = pipeline_cache
          .get_compute_pipeline(pipeline.update_pipeline)
          .unwrap();
        pass.set_pipeline(update_pipeline);
        pass.dispatch_workgroups(
          WINDOW_SIZE.0 / WORKGROUP_SIZE,
          WINDOW_SIZE.1 / WORKGROUP_SIZE,
          1,
        );
      }
    }

    Ok(())
  }
}
