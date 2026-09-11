/** 视口拾取：轨道相机指针射线与网格单元求交（纯函数，不依赖 WebGL 上下文）。 */
import type { Vec3 } from "./math";
import { cross, normalize, sub } from "./math";

/** 拾取所需的相机快照（与渲染循环同一套轨道参数）。 */
export interface CameraSnapshot {
  eye: Vec3;
  target: Vec3;
  /** 垂直视场角（弧度）。 */
  fovY: number;
  aspect: number;
  /** 画布 CSS 像素尺寸（指针坐标的参照系）。 */
  width: number;
  height: number;
}

interface PointerRay {
  origin: Vec3;
  dir: Vec3;
}

interface PickResult {
  /** 命中的面（渲染网格 indices 的面序号）。 */
  face: number;
  /** faceCells 映射后的单元索引（与场值同域）。 */
  cell: number;
  /** 射线参数 t（相机到命中点距离）。 */
  distance: number;
}

/** 指针坐标（相对画布左上角，CSS 像素）→ 世界射线。 */
export function rayFromPointer(camera: CameraSnapshot, x: number, y: number): PointerRay {
  const forward = normalize(sub(camera.target, camera.eye));
  const worldUp: Vec3 = [0, 1, 0];
  const right = normalize(cross(forward, worldUp));
  const up = normalize(cross(right, forward));
  const ndcX = (2 * x) / camera.width - 1;
  const ndcY = 1 - (2 * y) / camera.height;
  const tanHalf = Math.tan(camera.fovY / 2);
  const dir = normalize([
    forward[0] + ndcX * tanHalf * camera.aspect * right[0] + ndcY * tanHalf * up[0],
    forward[1] + ndcX * tanHalf * camera.aspect * right[1] + ndcY * tanHalf * up[1],
    forward[2] + ndcX * tanHalf * camera.aspect * right[2] + ndcY * tanHalf * up[2],
  ]);
  return { origin: camera.eye, dir };
}

export interface PickMesh {
  positions: Float32Array;
  indices: Uint32Array;
  faceCells: Uint32Array;
}

/** 逐面求交取最近命中；单元索引经 faceCells 映射（与云图着色同一映射）。 */
export function pickCell(mesh: PickMesh, ray: PointerRay): PickResult | null {
  let best: PickResult | null = null;
  const faceCount = Math.floor(mesh.indices.length / 3);
  for (let face = 0; face < faceCount; face += 1) {
    const ia = mesh.indices[face * 3];
    const ib = mesh.indices[face * 3 + 1];
    const ic = mesh.indices[face * 3 + 2];
    if (ia === undefined || ib === undefined || ic === undefined) {
      continue;
    }
    const a = vertexAt(mesh.positions, ia);
    const b = vertexAt(mesh.positions, ib);
    const c = vertexAt(mesh.positions, ic);
    const t = rayTriangle(ray.origin, ray.dir, a, b, c);
    if (t !== null && (best === null || t < best.distance)) {
      const cell = mesh.faceCells[face];
      if (cell === undefined) {
        continue;
      }
      best = { face, cell, distance: t };
    }
  }
  return best;
}

function vertexAt(positions: Float32Array, index: number): Vec3 {
  return [positions[index * 3] ?? 0, positions[index * 3 + 1] ?? 0, positions[index * 3 + 2] ?? 0];
}

/** Möller–Trumbore：射线与三角形相交的参数 t（t > 0），平行 / 背向返回 null。 */
function rayTriangle(origin: Vec3, dir: Vec3, a: Vec3, b: Vec3, c: Vec3): number | null {
  const e1 = sub(b, a);
  const e2 = sub(c, a);
  const p = cross(dir, e2);
  const det = dot(p, e1);
  // 平行或共面：容差随边长量级缩放，避免大坐标下的误判。
  if (Math.abs(det) < 1e-12) {
    return null;
  }
  const inv = 1 / det;
  const s = sub(origin, a);
  const u = dot(s, p) * inv;
  if (u < 0 || u > 1) {
    return null;
  }
  const q = cross(s, e1);
  const v = dot(q, dir) * inv;
  if (v < 0 || u + v > 1) {
    return null;
  }
  const t = dot(e2, q) * inv;
  return t > 1e-9 ? t : null;
}

function dot(a: Vec3, b: Vec3): number {
  return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}
