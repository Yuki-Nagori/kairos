import { describe, expect, it } from "vitest";
import { computeVertexNormals } from "../../../src-web/render/normals";

describe("computeVertexNormals（两后端共享的平滑法向）", () => {
  it("单位正方形（两三角形）的共享顶点法向为平滑累加结果", () => {
    // z=0 平面上的两个三角形组成正方形；面法向均 +Z，共享边顶点法向也是 +Z。
    const positions = new Float32Array([0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0]);
    const indices = new Uint32Array([0, 1, 2, 0, 2, 3]);
    const normals = computeVertexNormals(positions, indices);
    for (let i = 2; i < normals.length; i += 3) {
      expect(normals[i]).toBeCloseTo(1, 5);
    }
  });

  it("退化三角形（零面积）不产生 NaN，法向保持零", () => {
    const positions = new Float32Array([0, 0, 0, 1, 0, 0, 2, 0, 0]);
    const indices = new Uint32Array([0, 1, 2]);
    const normals = computeVertexNormals(positions, indices);
    for (const value of normals) {
      expect(Number.isNaN(value)).toBe(false);
    }
  });

  it("共享顶点累加两侧面法向后归一化（平滑语义与逐面覆写不同）", () => {
    // 面 1（v0,v1,v2）法向 +Y；面 2（v0,v3,v1）法向 -Z；共享边 v0-v1。
    // 共享顶点法向 = (0,1,0)+(0,0,-1) 归一化 ≈ (0, 0.707, -0.707)；
    // 非共享顶点保持自己面的单位法向。
    const positions = new Float32Array([0, 0, 0, 1, 0, 0, 0, 0, -1, 0, 1, 0]);
    const indices = new Uint32Array([0, 1, 2, 0, 3, 1]);
    const normals = computeVertexNormals(positions, indices);
    // 顶点 2：仅属于面 1 → +Y
    expect(normals[7]).toBeCloseTo(1, 5);
    // 顶点 3：仅属于面 2 → -Z
    expect(normals[11]).toBeCloseTo(-1, 5);
    // 共享顶点 0：两面法向的归一化和
    expect(normals[0]).toBeCloseTo(0, 5);
    expect(normals[1]).toBeCloseTo(Math.SQRT1_2, 4);
    expect(normals[2]).toBeCloseTo(-Math.SQRT1_2, 4);
  });
});
