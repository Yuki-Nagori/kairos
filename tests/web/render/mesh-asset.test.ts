/** 确定性基准资产生成器单测。 */
import { describe, expect, it } from "vitest";
import { deterministicGridMesh } from "../../../src-web/render/webgpu/mesh-asset";

describe("deterministicGridMesh", () => {
  it("按 2 × n² 生成合法拓扑", () => {
    const mesh = deterministicGridMesh(8);
    expect(mesh.indices.length).toBe(8 * 3);
    expect(mesh.faceCells.length).toBe(8);
    expect(mesh.positions.length).toBe(9 * 3);
  });

  it("同一参数两次生成结果逐字节一致（确定性）", () => {
    const a = deterministicGridMesh(512);
    const b = deterministicGridMesh(512);
    expect(a.positions).toEqual(b.positions);
    expect(a.indices).toEqual(b.indices);
    expect(a.faceCells).toEqual(b.faceCells);
    // 大资产（1M 三角形量级）只校验规模，避免深比较的长耗时。
    const big = deterministicGridMesh(2 * 707 * 707);
    expect(big.faceCells.length).toBe(999_698);
    expect(big.positions.length).toBe(708 * 708 * 3);
  });

  it("拒绝奇数与负数", () => {
    expect(() => deterministicGridMesh(3)).toThrow("正偶数");
    expect(() => deterministicGridMesh(-2)).toThrow("正偶数");
    expect(() => deterministicGridMesh(6)).toThrow("2 × n²");
  });

  it("面值索引覆盖全部面且包围盒归一化", () => {
    const mesh = deterministicGridMesh(2 * 4 * 4);
    expect(Array.from(mesh.faceCells)).toEqual([...Array(32).keys()]);
    const xs = mesh.positions.filter((_, i) => i % 3 === 0);
    expect(Math.min(...xs)).toBe(0);
    expect(Math.max(...xs)).toBe(1);
  });
});
