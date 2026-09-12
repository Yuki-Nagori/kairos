/**
 * 每顶点平滑法向：三角形面法向按共享顶点累加后逐顶点归一化。
 * WebGL2 回退后端与 WebGPU 主路径共用同一实现——两后端对同一网格
 * 必须呈现一致的明暗；此前一边累加（平滑）、一边逐面覆写（平面），
 * 同一模型在两个后端下观感不同。
 */

export function computeVertexNormals(positions: Float32Array, indices: Uint32Array): Float32Array {
  const normals = new Float32Array(positions.length);
  for (let face = 0; face < indices.length; face += 3) {
    const a = (indices[face] ?? 0) * 3;
    const b = (indices[face + 1] ?? 0) * 3;
    const c = (indices[face + 2] ?? 0) * 3;
    const e1x = (positions[b] ?? 0) - (positions[a] ?? 0);
    const e1y = (positions[b + 1] ?? 0) - (positions[a + 1] ?? 0);
    const e1z = (positions[b + 2] ?? 0) - (positions[a + 2] ?? 0);
    const e2x = (positions[c] ?? 0) - (positions[a] ?? 0);
    const e2y = (positions[c + 1] ?? 0) - (positions[a + 1] ?? 0);
    const e2z = (positions[c + 2] ?? 0) - (positions[a + 2] ?? 0);
    const nx = e1y * e2z - e1z * e2y;
    const ny = e1z * e2x - e1x * e2z;
    const nz = e1x * e2y - e1y * e2x;
    for (const base of [a, b, c]) {
      normals[base] = (normals[base] ?? 0) + nx;
      normals[base + 1] = (normals[base + 1] ?? 0) + ny;
      normals[base + 2] = (normals[base + 2] ?? 0) + nz;
    }
  }
  for (let index = 0; index < normals.length; index += 3) {
    const length = Math.hypot(
      normals[index] ?? 0,
      normals[index + 1] ?? 0,
      normals[index + 2] ?? 0,
    );
    if (length > 0) {
      normals[index] = (normals[index] ?? 0) / length;
      normals[index + 1] = (normals[index + 1] ?? 0) / length;
      normals[index + 2] = (normals[index + 2] ?? 0) / length;
    }
  }
  return normals;
}
