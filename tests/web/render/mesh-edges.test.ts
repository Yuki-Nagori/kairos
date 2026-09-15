import { describe, expect, it } from "vitest";
import { meshEdgeIndices } from "../../../src-web/render/mesh-edges";

describe("网格边线 GPU 索引", () => {
  it("保留共享顶点和超过 u16 范围的节点编号，闭合每个三角形", () => {
    const triangles = new Uint32Array([70000, 2, 9, 9, 2, 4]);
    expect([...meshEdgeIndices(triangles)]).toEqual([70000, 2, 2, 9, 9, 70000, 9, 2, 2, 4, 4, 9]);
    expect([...triangles]).toEqual([70000, 2, 9, 9, 2, 4]);
  });
  it("空网格没有边线", () => {
    expect(meshEdgeIndices(new Uint32Array())).toHaveLength(0);
  });
});
