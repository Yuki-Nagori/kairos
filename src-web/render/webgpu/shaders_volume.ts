/** 体渲染 WGSL（T50 POC）：全屏三角形 + 固定步长光线步进，
 * 体素场三线性采样，前向合成 alpha 混色。
 * 射线由 uniform 传入的轨道相机基向量 + 视口尺寸逐像素构造（无逆矩阵）。
 * uniform 布局（vec4 成员，各 16 字节对齐）：
 *   [0]  dims_pad        = (nx, ny, nz, 0)
 *   [1]  origin          = (ox, oy, oz, 0)
 *   [2]  spacing         = (hx, hy, hz, 0)
 *   [3]  eye             = (ex, ey, ez, 0)
 *   [4]  right           = (rx, ry, rz, 0)
 *   [5]  up              = (ux, uy, uz, 0)
 *   [6]  forward         = (fx, fy, fz, 0)
 *   [7]  viewport        = (宽, 高, aspect, tan(fov/2))
 *   [8]  scalars         = (value_max, step_alpha, steps, 0)
 */

export const VOLUME_UNIFORM_FLOATS = 32;

export const VOLUME_VERTEX_SHADER = /* wgsl */ `
struct Uniforms {
  dims_pad: vec4<u32>,
  origin: vec4<f32>,
  spacing: vec4<f32>,
  eye: vec4<f32>,
  right: vec4<f32>,
  up: vec4<f32>,
  forward: vec4<f32>,
  viewport: vec4<f32>,
  scalars: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

@vertex
fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
  var corners = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(3.0, -1.0),
    vec2<f32>(-1.0, 3.0),
  );
  return vec4<f32>(corners[index], 0.0, 1.0);
}
`;

export const VOLUME_FRAGMENT_SHADER = /* wgsl */ `
struct Uniforms {
  dims_pad: vec4<u32>,
  origin: vec4<f32>,
  spacing: vec4<f32>,
  eye: vec4<f32>,
  right: vec4<f32>,
  up: vec4<f32>,
  forward: vec4<f32>,
  viewport: vec4<f32>,
  scalars: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<storage, read> values: array<f32>;

fn sampleGrid(p: vec3<f32>) -> f32 {
  let grid = (p - u.origin.xyz) / u.spacing.xyz - vec3<f32>(0.5);
  let base = floor(max(grid, vec3<f32>(0.0)));
  let frac = grid - base;
  var acc = 0.0;
  for (var corner = 0u; corner < 8u; corner = corner + 1u) {
    let offset = vec3<f32>(
      f32(corner & 1u),
      f32((corner >> 1u) & 1u),
      f32((corner >> 2u) & 1u),
    );
    let max_cell = vec3<f32>(u.dims_pad.xyz) - vec3<f32>(1.0);
    let cell = min(base + offset, max_cell);
    let flat = cell.x + u.dims_pad.x * (cell.y + u.dims_pad.y * cell.z);
    let weight = select(1.0 - frac.x, frac.x, offset.x > 0.5)
      * select(1.0 - frac.y, frac.y, offset.y > 0.5)
      * select(1.0 - frac.z, frac.z, offset.z > 0.5);
    acc = acc + values[u32(flat)] * weight;
  }
  return acc;
}

fn rayBox(origin: vec3<f32>, dir: vec3<f32>) -> vec2<f32> {
  let lo = u.origin.xyz;
  let hi = lo + vec3<f32>(u.dims_pad.xyz) * u.spacing.xyz;
  var t_near = -1e30;
  var t_far = -1e30;
  var hit = true;
  for (var axis = 0; axis < 3; axis = axis + 1) {
    let o = origin[axis];
    let d = dir[axis];
    let lo_axis = lo[axis];
    let hi_axis = hi[axis];
    if (abs(d) < 1e-12) {
      if (o < lo_axis || o > hi_axis) {
        hit = false;
      }
    } else {
      let t0 = (lo_axis - o) / d;
      let t1 = (hi_axis - o) / d;
      t_near = max(t_near, min(t0, t1));
      t_far = max(t_far, max(t0, t1));
    }
  }
  if (!hit || t_near > t_far || t_far <= 0.0) {
    return vec2<f32>(1e30, -1e30);
  }
  return vec2<f32>(max(t_near, 0.0), t_far);
}

@fragment
fn fs(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
  let ndc = vec2<f32>(
    (2.0 * frag_coord.x) / u.viewport.x - 1.0,
    1.0 - (2.0 * frag_coord.y) / u.viewport.y,
  );
  let dir = normalize(
    u.forward.xyz + ndc.x * u.viewport.z * u.viewport.w * u.right.xyz
      + ndc.y * u.viewport.w * u.up.xyz,
  );

  let range = rayBox(u.eye.xyz, dir);
  if (range.x > range.y) {
    discard;
  }
  let steps = u.scalars.z;
  let step_size = (range.y - range.x) / steps;
  let step_alpha = clamp(u.scalars.y, 0.001, 0.5);

  var alpha_acc = 0.0;
  var color_acc = vec3<f32>(0.0);
  for (var step_index = 0u; step_index < steps; step_index = step_index + 1u) {
    let t = range.x + (f32(step_index) + 0.5) * step_size;
    let p = u.eye.xyz + dir * t;
    let value = sampleGrid(p);
    if (value > 1e-6) {
      let normalized = clamp(value / u.scalars.x, 0.0, 1.0);
      // 冷（蓝）→ 热（红）迁移函数；不透明度随值升高。
      let color = mix(vec3<f32>(0.16, 0.38, 0.85), vec3<f32>(0.95, 0.45, 0.15), normalized);
      let alpha = clamp(step_alpha * (0.3 + normalized), 0.0, 1.0);
      color_acc = color_acc + (1.0 - alpha_acc) * alpha * color;
      alpha_acc = alpha_acc + (1.0 - alpha_acc) * alpha;
      if (alpha_acc > 0.99) {
        break;
      }
    }
  }
  if (alpha_acc < 1e-4) {
    discard;
  }
  return vec4<f32>(color_acc, alpha_acc);
}
`;
