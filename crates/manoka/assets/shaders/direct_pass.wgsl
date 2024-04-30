
@compute @workgroup_size(8, 8, 8)
fn update(
  @builtin(global_invocation_id) invocation_id: vec3<u32>,
  @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
  // nothing happens here
}
