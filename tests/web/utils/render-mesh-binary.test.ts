import { describe, it, expect } from "vitest";
import { decodeRenderMesh } from "../../../src-web/utils/render-mesh-binary";

describe("render mesh codec", () => {
  it("reads array views without copying the payload", () => {
    const buffer = new ArrayBuffer(44);
    const view = new DataView(buffer);
    view.setUint32(0, 0x314d524b, true);
    view.setUint32(4, 3, true);
    view.setUint32(8, 3, true);
    view.setUint32(12, 1, true);
    view.setFloat32(16, 1.5, true);
    view.setUint32(40, 7, true);
    const mesh = decodeRenderMesh(buffer);
    expect(Array.from(mesh.positions)).toEqual([1.5, 0, 0]);
    expect(Array.from(mesh.indices)).toEqual([0, 0, 0]);
    expect(Array.from(mesh.faceCells)).toEqual([7]);
    expect((mesh.positions as Float32Array).buffer).toBe(buffer);
  });
  it("rejects truncated, foreign and mismatched payloads", () => {
    expect(() => decodeRenderMesh(new ArrayBuffer(3))).toThrow("头无效");
    expect(() => decodeRenderMesh(new ArrayBuffer(16))).toThrow("头无效");
    const buffer = new ArrayBuffer(16);
    const view = new DataView(buffer);
    view.setUint32(0, 0x314d524b, true);
    view.setUint32(4, 3, true);
    expect(() => decodeRenderMesh(buffer)).toThrow("长度不符");
  });
});
