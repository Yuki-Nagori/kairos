/** 拾取纯函数单测：射线构造与网格求交（不依赖 WebGL 上下文）。 */
import { describe, expect, it } from "vitest";
import { pickCell, rayFromPointer, type CameraSnapshot } from "../../../src-web/render/picking";

const camera: CameraSnapshot = {
  eye: [0, 0, 5],
  target: [0, 0, 0],
  fovY: Math.PI / 4,
  aspect: 1,
  width: 800,
  height: 600,
};

/** z = 0 平面上的两个三角形：面 0 覆盖 x ∈ [-1, 0]，面 1 覆盖 x ∈ [0, 1]。 */
const mesh = {
  positions: new Float32Array([
    -1,
    -1,
    0,
    0,
    -1,
    0,
    0,
    1,
    0, //
    -1,
    -1,
    0,
    0,
    1,
    0,
    -1,
    1,
    0,
  ]),
  indices: new Uint32Array([0, 1, 2, 3, 4, 5]),
  faceCells: new Uint32Array([7, 3]),
};

describe("rayFromPointer", () => {
  it("画布中心射线沿视线方向（-Z）", () => {
    const ray = rayFromPointer(camera, 400, 300);
    expect(ray.origin).toEqual([0, 0, 5]);
    expect(ray.dir[0]).toBeCloseTo(0, 6);
    expect(ray.dir[1]).toBeCloseTo(0, 6);
    expect(ray.dir[2]).toBeCloseTo(-1, 6);
  });

  it("右半画布的射线偏向 +X，上方画布的射线偏向 +Y", () => {
    const right = rayFromPointer(camera, 700, 300);
    expect(right.dir[0]).toBeGreaterThan(0);
    const top = rayFromPointer(camera, 400, 100);
    expect(top.dir[1]).toBeGreaterThan(0);
  });
});

describe("pickCell", () => {
  it("命中面并映射到单元索引", () => {
    const ray = rayFromPointer(camera, 400, 300);
    const hit = pickCell(mesh, ray);
    expect(hit).not.toBeNull();
    expect(hit?.face).toBe(0);
    expect(hit?.cell).toBe(7);
    expect(hit?.distance).toBeCloseTo(5, 6);
  });

  it("偏移命中返回另一单元", () => {
    // 两个 1×1 方格（各拆两三角）：x<0 为单元 7，x>0 为单元 3；
    // 对角线从 (0,0) 到 (1,1)，取偏离对角线的指针位置避免边界歧义。
    const twoCells = {
      positions: new Float32Array([
        0,
        0,
        0,
        1,
        0,
        0,
        1,
        1,
        0, //
        0,
        0,
        0,
        1,
        1,
        0,
        0,
        1,
        0, //
        -1,
        0,
        0,
        0,
        0,
        0,
        0,
        1,
        0, //
        -1,
        0,
        0,
        0,
        1,
        0,
        -1,
        1,
        0,
      ]),
      indices: new Uint32Array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
      faceCells: new Uint32Array([7, 7, 3, 3]),
    };
    // 相机对准两方格交界处。
    const centered: CameraSnapshot = { ...camera, eye: [0, 0, 5], target: [0, 0, 0] };
    const left = pickCell(twoCells, rayFromPointer(centered, 300, 300));
    expect(left?.cell).toBe(3);
    const right = pickCell(twoCells, rayFromPointer(centered, 500, 300));
    expect(right?.cell).toBe(7);
  });

  it("背向与脱空射线返回 null", () => {
    // 射线背向网格（朝 +Z 远离 z=0 平面）。
    const behind = pickCell(mesh, { origin: [0, 0, 5], dir: [0, 0, 1] });
    expect(behind).toBeNull();
    // 射线完全脱空。
    const miss = pickCell(mesh, { origin: [0, 0, 5], dir: [0, 1, -0.0001] });
    expect(miss).toBeNull();
  });
});
