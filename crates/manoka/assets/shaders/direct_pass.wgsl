
const CHUNK_VOXEL_COUNT: u32 = 64*64*64;
const CHUNK_VOXEL_COUNT_DIV_32: u32 = CHUNK_VOXEL_COUNT / 32;

struct ChunkOccupancy {
  occupancy: array<u32, CHUNK_VOXEL_COUNT_DIV_32>,
}

struct SunLight {
  color:       vec4<f32>,
  illuminance: f32,
  direction:   vec3<f32>,
}

struct FullVoxel {
  normal: vec3<f32>,
  color: vec3<f32>,
}

struct ChunkAttributes {
  attributes: array<FullVoxel>,
}

struct DirectPassUniform {
  current_chunk: u32,
}

struct DirectPassOutput {
  output: array<vec3<f32>, CHUNK_VOXEL_COUNT>,
}

@group(0) @binding(0) var<storage> om_array: array<ChunkOccupancy>; 
@group(0) @binding(1) var<storage> transform_array: array<mat4x4<f32>>;
@group(0) @binding(2) var<storage> light_array: array<SunLight>;

@group(1) @binding(0) var<storage> attributes: ChunkAttributes;
@group(1) @binding(1) var<uniform> current: DirectPassUniform;
@group(1) @binding(2) var<storage, read_write> output: DirectPassOutput;

@compute @workgroup_size(4, 4, 4)
fn update(
  @builtin(global_invocation_id) invocation_id: vec3<u32>,
  @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
  let index = invocation_id.z * 64 * 64 + invocation_id.y * 64 + invocation_id.x;
  output.output[index] = vec3<f32>(invocation_id) / 64;
}
