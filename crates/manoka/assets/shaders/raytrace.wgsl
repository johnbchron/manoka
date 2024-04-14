
struct Camera {
  pos: vec3<f32>,
  look_at: vec3<f32>,
  fov: f32,
  up: vec3<f32>,
};

struct Voxel {
  material: u32,
  normal: vec3<f32>,
};

struct Ray {
  origin: vec3<f32>,
  dir: vec3<f32>,
};

struct RayHit {
  hit: bool,
  point: vec3<f32>,
  normal: vec3<f32>,
  chunk_coords: vec3<u32>,
  steps: u32,
  through_empty: u32,
};

const VOXEL_COUNT: u32 = 64*64*64;

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<storage> voxels: array<Voxel>; 
@group(0) @binding(2) var output_texture: texture_storage_2d<rgba8unorm, read_write>;

@compute @workgroup_size(32, 32, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
  let screen_coords: vec2<u32> = invocation_id.xy;

  var output_color: vec3<f32> = vec3(0.0);

  let ray_dir: vec3<f32> = calculate_ray_dir(invocation_id);
  let ray = Ray(camera.pos, ray_dir);
  let hit = cast_ray(ray);

  if (hit.hit) {
    output_color = hit.normal;
  } else {
    output_color = ray_dir / 2.0 + 0.5;
  }

  storageBarrier();

  textureStore(output_texture, screen_coords, vec4(output_color, 1.0));
}

// https://raytracing.github.io/books/RayTracingInOneWeekend.html
fn calculate_ray_dir(invocation_id: vec3<u32>) -> vec3<f32> {
  let screen_coords: vec2<f32> = vec2<f32>(invocation_id.xy);
  let screen_size: vec2<f32> = vec2<f32>(textureDimensions(output_texture));

  let aspect_ratio = screen_size.x / screen_size.y;

  let focal_length = length(camera.pos - camera.look_at);
  let theta = camera.fov;
  let h = tan(theta/2);
  let viewport_height = 2 * h * focal_length;
  let viewport_width = viewport_height * aspect_ratio;

  // Calculate the u,v,w unit basis vectors for the camera coordinate frame.
  let w = -normalize(camera.pos - camera.look_at);
  let u = normalize(cross(camera.up, w));
  let v = -cross(w, u);

  // Calculate the vectors across the horizontal and down the vertical viewport edges.
  let viewport_u = viewport_width * u;
  let viewport_v = viewport_height * -v;

  // Calculate the horizontal and vertical delta vectors from pixel to pixel.
  let pixel_delta_u = viewport_u / screen_size.x;
  let pixel_delta_v = viewport_v / screen_size.y;

  // Calculate the location of the upper left pixel.
  let viewport_upper_left = camera.pos - (focal_length * w) - viewport_u/2 - viewport_v/2;
  let pixel00_loc = viewport_upper_left + 0.5 * (pixel_delta_u + pixel_delta_v);

  // Calculate the location of *this* pixel.
  let current_pixel_center = pixel00_loc + (screen_coords.x * pixel_delta_u) + (screen_coords.y * pixel_delta_v);
  return normalize(camera.pos - current_pixel_center);
}

fn is_valid_voxel(coords: vec3<u32>) -> bool {
  return all(coords < vec3(64));
}

fn sample_voxel(coords: vec3<u32>) -> Voxel {
  return voxels[coords.x + coords.y*64 + coords.z*64*64];
}

fn cast_ray(ray: Ray) -> RayHit {
  var hit: RayHit = RayHit(false, vec3(0.0), vec3(0.0), vec3(0), 0, 0);

  var p: vec3<f32> = floor(ray.origin) + 0.5;

	let dRd = 1.0 / max(abs(ray.dir), vec3(0.00001));

	let rds = sign(ray.dir);
  var side: vec3<f32> = dRd * (rds * (p - ray.origin) + 0.5);
    
  var mask: vec3<f32> = vec3(0.0);
	
  var i: u32 = 0;
	loop {
    // exit if we haven't converged in time
    if (i > 100) { break; } else { i++; }

    hit.point = p;
    hit.chunk_coords = vec3<u32>(p + 32);
    hit.steps = i;

    // see if we've converged
    if (is_valid_voxel(hit.chunk_coords)) {
      let voxel = sample_voxel(hit.chunk_coords);
      if (voxel.material == 1) {
        hit.hit = true;
        // hit.normal = mask * sign(ray.dir);
        hit.normal = voxel.normal;
        break;
      } else {
        hit.through_empty = hit.through_empty + 1;
      }
    }

    // step through
    mask = step(side, side.yzx) * (1.0 - step(side.zxy, side));
    side += mask * dRd;
    p += mask * rds;
	}
    
  return hit;
}
