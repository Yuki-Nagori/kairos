/** WebGPU POC 着色器：与 WebGL2 主后端同一套视觉语义
 * （云图冷热映射 + 平行光漫反射 + 平面剖切丢弃）。 */

export const MESH_VERTEX_SHADER = /* wgsl */ `
struct Uniforms {
  mvp: mat4x4<f32>,
  light_dir: vec3<f32>,
  clip_offset: f32,
  clip_normal: vec3<f32>,
  clip_enabled: f32,
  use_field: f32,
  value_min: f32,
  value_max: f32,
  pad: f32,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VSOut {
  @builtin(position) position: vec4<f32>,
  @location(0) world: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) value: f32,
};

@vertex
fn vs(@location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) value: f32) -> VSOut {
  var out: VSOut;
  out.position = u.mvp * vec4<f32>(pos, 1.0);
  out.world = pos;
  out.normal = normal;
  out.value = value;
  return out;
}
`;

export const MESH_FRAGMENT_SHADER = /* wgsl */ `
struct Uniforms {
  mvp: mat4x4<f32>,
  light_dir: vec3<f32>,
  clip_offset: f32,
  clip_normal: vec3<f32>,
  clip_enabled: f32,
  use_field: f32,
  value_min: f32,
  value_max: f32,
  pad: f32,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VSOut {
  @builtin(position) position: vec4<f32>,
  @location(0) world: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) value: f32,
};

@fragment
fn fs(in: VSOut) -> @location(0) vec4<f32> {
  if (u.clip_enabled > 0.5 && dot(in.world, u.clip_normal) > u.clip_offset) {
    discard;
  }
  let n = normalize(in.normal);
  let diff = max(dot(n, normalize(u.light_dir)), 0.0);
  let t = clamp((in.value - u.value_min) / max(u.value_max - u.value_min, 1e-6), 0.0, 1.0);
  let field = mix(vec3<f32>(0.05, 0.33, 0.66), vec3<f32>(0.94, 0.33, 0.13), t);
  let base = select(vec3<f32>(0.55, 0.58, 0.62), field, u.use_field > 0.5);
  return vec4<f32>(base * (0.35 + 0.65 * diff), 1.0);
}
`;

export const LINE_VERTEX_SHADER = /* wgsl */ `
struct LineUniforms {
  mvp: mat4x4<f32>,
  color: vec4<f32>,
};
@group(0) @binding(0) var<uniform> lu: LineUniforms;

@vertex
fn vs(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
  return lu.mvp * vec4<f32>(pos, 1.0);
}
`;

export const LINE_FRAGMENT_SHADER = /* wgsl */ `
struct LineUniforms {
  mvp: mat4x4<f32>,
  color: vec4<f32>,
};
@group(0) @binding(0) var<uniform> lu: LineUniforms;

@fragment
fn fs() -> @location(0) vec4<f32> {
  return lu.color;
}
`;
