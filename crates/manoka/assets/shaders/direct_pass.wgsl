
const CHUNK_VOXEL_COUNT: u32 = 64*64*64;
const CHUNK_VOXEL_COUNT_DIV_32: u32 = CHUNK_VOXEL_COUNT / 32;

struct ChunkOccupancy {
  occupancy: array<u32, CHUNK_VOXEL_COUNT_DIV_32>,
}

struct ChunkAttributes {
  attributes: array<FullVoxel>,
}

struct DirectPassOutput {
  output: array<vec3<f32>, CHUNK_VOXEL_COUNT>,
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

@group(0) @binding(0) var<storage> om_buffers: binding_array<ChunkOccupancy>; 
@group(0) @binding(1) var<storage> attribute_buffers: binding_array<ChunkAttributes>;
@group(0) @binding(2) var<storage, read_write> output_buffers: binding_array<DirectPassOutput>;
@group(0) @binding(3) var<storage> transform_array: array<mat4x4<f32>>;
@group(0) @binding(4) var<storage> light_array: array<SunLight>;

@compute @workgroup_size(4, 4, 4)
fn update(
  @builtin(global_invocation_id) invocation_id: vec3<u32>,
  @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
  let chunk_index = (invocation_id.z - invocation_id.z % 64) / 64;
  let index = (invocation_id.z % 64) * 64 * 64 + invocation_id.y * 64 + invocation_id.x;
  output_buffers[chunk_index].output[index] = vec3<f32>(invocation_id % 64) / 64;
}
