import { describe, expect, it } from "vitest";
import {
  clipPlaneFromFraction,
  fitCameraToBounds,
  cross,
  type Mat4,
  dot,
  mat4Identity,
  mat4LookAt,
  mat4Multiply,
  mat4Perspective,
  mat4RotateX,
  mat4RotateY,
  normalize,
  sub,
} from "../../../src-web/render/math";
import type { Vec3 } from "../../../src-web/render/math";

describe("math", () => {
  it("identity leaves points unchanged", () => {
    const m = mat4Identity();
    // 变换 (1,2,3,1)
    const x = m[0]! * 1 + m[4]! * 2 + m[8]! * 3 + m[12]!;
    const y = m[1]! * 1 + m[5]! * 2 + m[9]! * 3 + m[13]!;
    const z = m[2]! * 1 + m[6]! * 2 + m[10]! * 3 + m[14]!;
    expect([x, y, z]).toEqual([1, 2, 3]);
  });

  it("multiply combines translations", () => {
    const translate = (tx: number, ty: number, tz: number): Mat4 =>
      new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, tx, ty, tz, 1]);
    const a = translate(1, 0, 0);
    const b = translate(0, 2, 0);
    const combined = mat4Multiply(a, b);
    // 对原点生效： combined * (0,0,0,1) = (1,2,0)
    const x = combined[12] ?? 0;
    const y = combined[13] ?? 0;
    expect([x, y]).toEqual([1, 2]);
  });

  it("perspective maps near plane correctly", () => {
    const p = mat4Perspective(Math.PI / 2, 1, 1, 11);
    // 近平面中心 (0,0,-1) → ndc z 应为 -1
    const z = (p[10] ?? 0) * -1 + (p[14] ?? 0);
    const w = 1;
    expect(z / w).toBeCloseTo(-1, 5);
    // 远平面中心 (0,0,-11) → ndc z 应为 +1
    const farZ = (p[10] ?? 0) * -11 + (p[14] ?? 0);
    expect(farZ / 11).toBeCloseTo(1, 5);
  });

  it("lookAt orients z axis toward the eye", () => {
    const view = mat4LookAt([0, 0, 5], [0, 0, 0], [0, 1, 0]);
    // 位于 +z 的相机看向原点：点 (0,0,0) 经 view 后 z = -5
    const z = (view[2] ?? 0) * 0 + (view[6] ?? 0) * 0 + (view[10] ?? 0) * 0 + (view[14] ?? 0);
    expect(z).toBeCloseTo(-5, 6);
  });

  it("vector helpers", () => {
    expect(normalize([3, 0, 0])).toEqual([1, 0, 0]);
    expect(cross([1, 0, 0], [0, 1, 0])).toEqual([0, 0, 1]);
    expect(dot([1, 2, 3], [4, 5, 6])).toBe(32);
    expect(sub([5, 7, 9], [1, 2, 3])).toEqual([4, 5, 6]);
  });

  it("rotation matrices keep length", () => {
    const rotY = mat4RotateY(Math.PI / 3);
    const rotX = mat4RotateX(Math.PI / 6);
    // 旋转矩阵不改变 x 基向量长度
    const lenX = Math.hypot(rotY[0] ?? 0, rotY[1] ?? 0, rotY[2] ?? 0);
    expect(lenX).toBeCloseTo(1, 6);
    const lenY = Math.hypot(rotX[4] ?? 0, rotX[5] ?? 0, rotX[6] ?? 0);
    expect(lenY).toBeCloseTo(1, 6);
  });

  it("clipPlaneFromFraction maps fraction to plane offset along axis", () => {
    const min: Vec3 = [0, 0, 0];
    const max: Vec3 = [10, 20, 30];
    const plane = clipPlaneFromFraction(min, max, "y", 0.25, false);
    expect(plane.normal).toEqual([0, 1, 0]);
    expect(plane.offset).toBeCloseTo(5, 9);
    // 反向：法向翻转，保留另一侧。
    const inverted = clipPlaneFromFraction(min, max, "y", 0.25, true);
    expect(inverted.normal).toEqual([0, -1, 0]);
    expect(inverted.offset).toBeCloseTo(-5, 9);
    // 分数越界被夹取到 0..1。
    expect(clipPlaneFromFraction(min, max, "z", 2, false).offset).toBeCloseTo(30, 9);
    expect(clipPlaneFromFraction(min, max, "z", -1, false).offset).toBeCloseTo(0, 9);
  });
});

describe("fitCameraToBounds（逐轴适配，回归锁定）", () => {
  it("不居中的网格按逐轴中点取注视点（修复三轴共用 min/max 的回归）", () => {
    // x ∈ [10,11]、y ∈ [0,100]、z ∈ [0,1]：旧单轴算法会把三轴中心都算错
    const fit = fitCameraToBounds({ min: [10, 0, 0], max: [11, 100, 1] }, [0, 1, 0]);
    expect(fit.target).toEqual([10.5, 50, 0.5]);
    expect(fit.distance).toBe(250); // 最大轴跨度 100 × 2.5
    expect(fit.clipOffset).toBe(50); // dot(center, [0,1,0])
  });

  it("退化包围盒（单点）跨度取 1e-6 下限", () => {
    const fit = fitCameraToBounds({ min: [3, 4, 5], max: [3, 4, 5] }, [0, 0, 1]);
    expect(fit.distance).toBeCloseTo(2.5e-6, 12);
    expect(fit.target).toEqual([3, 4, 5]);
    expect(fit.clipOffset).toBe(5);
  });

  it("斜剖切法向的偏移为注视点在法向上的投影", () => {
    const fit = fitCameraToBounds({ min: [0, 0, 0], max: [2, 2, 2] }, [
      0,
      Math.SQRT1_2,
      Math.SQRT1_2,
    ]);
    expect(fit.clipOffset).toBeCloseTo(2 * Math.SQRT1_2, 12);
  });
});
