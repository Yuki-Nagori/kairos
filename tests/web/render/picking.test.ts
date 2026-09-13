/** 拾取纯函数单测：射线构造与网格求交（不依赖 WebGL 上下文）。 */
import { describe, expect, it } from "vitest";
import {
  pickCell,
  rayFromPointer,
  snapToNode,
  type CameraSnapshot,
} from "../../../src-web/render/picking";

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
  it("命中面并映射到单元索引，同时给出命中点世界坐标", () => {
    const ray = rayFromPointer(camera, 400, 300);
    const hit = pickCell(mesh, ray);
    expect(hit).not.toBeNull();
    expect(hit?.face).toBe(0);
    expect(hit?.cell).toBe(7);
    expect(hit?.distance).toBeCloseTo(5, 6);
    // 指针在画布中心 → 命中点在 z = 0 平面上、接近三角形缺省重心。
    expect(hit?.point[2]).toBeCloseTo(0, 6);
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

describe("snapToNode", () => {
  /** z = 0 平面上一个直角三角形：顶点 (0,0) (1,0) (0,1)。 */
  const tri = {
    positions: new Float32Array([0, 0, 0, 1, 0, 0, 0, 1, 0]),
    indices: new Uint32Array([0, 1, 2]),
    faceCells: new Uint32Array([0]),
  };

  it("吸附到命中三角形的最近顶点", () => {
    const nearOrigin = snapToNode(tri, {
      face: 0,
      cell: 0,
      distance: 1,
      point: [0.05, 0.02, 0],
    });
    expect(nearOrigin).toEqual([0, 0, 0]);

    const nearX = snapToNode(tri, {
      face: 0,
      cell: 0,
      distance: 1,
      point: [0.9, 0.05, 0],
    });
    expect(nearX).toEqual([1, 0, 0]);

    // 顶点坐标本身命中时精确返回该顶点。
    const exact = snapToNode(tri, { face: 0, cell: 0, distance: 1, point: [0, 1, 0] });
    expect(exact).toEqual([0, 1, 0]);
  });

  it("无节点数据时回退命中单元中心", () => {
    // 命中三角形的顶点坐标缺失，但同单元的另一面（面 1）可解析 → 用该单元顶点均值。
    const broken = {
      positions: new Float32Array([0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 1, 0]),
      indices: new Uint32Array([9, 10, 11, 0, 1, 3]), // 面 0 的三个顶点索引都越界
      faceCells: new Uint32Array([5, 5]),
    };
    const center = snapToNode(broken, { face: 0, cell: 5, distance: 1, point: [0.5, 0.5, 0] });
    // 面 1 的顶点 (0,0,0) (1,0,0) (1,1,0) → 均值 (2/3, 1/3, 0)
    expect(center[0]).toBeCloseTo(2 / 3, 12);
    expect(center[1]).toBeCloseTo(1 / 3, 12);
    expect(center[2]).toBe(0);
  });

  it("节点与单元中心都取不到时回退命中点", () => {
    const empty = {
      positions: new Float32Array([]),
      indices: new Uint32Array([0, 1, 2]),
      faceCells: new Uint32Array([0]),
    };
    expect(snapToNode(empty, { face: 0, cell: 0, distance: 1, point: [0.5, 0.5, 0] })).toEqual([
      0.5, 0.5, 0,
    ]);

    // 面号越界 → 无命中面顶点，但单元 0 仍有面可聚合 → 单元中心 (1/3, 1/3, 0)
    const center = snapToNode(tri, { face: 9, cell: 0, distance: 1, point: [1, 2, 3] });
    expect(center[0]).toBeCloseTo(1 / 3, 12);
    expect(center[1]).toBeCloseTo(1 / 3, 12);
    expect(center[2]).toBe(0);

    // 单元号在 faceCells 中不存在 → 单元中心为 null，回到命中点
    expect(snapToNode(empty, { face: 0, cell: 42, distance: 1, point: [7, 8, 9] })).toEqual([
      7, 8, 9,
    ]);
  });

  it("部分顶点坐标缺失时只在可用顶点中选择", () => {
    // 三角形 1 号索引的坐标缺失：候选只剩 (0,0,0) 与 (0,1,0)。
    const partial = {
      positions: new Float32Array([0, 0, 0, 1, 0]), // 缺 1 号顶点的 z
      indices: new Uint32Array([0, 1, 2]),
      faceCells: new Uint32Array([0]),
    };
    expect(snapToNode(partial, { face: 0, cell: 0, distance: 1, point: [0.9, 0.1, 0] })).toEqual([
      0, 0, 0,
    ]);
  });
});
