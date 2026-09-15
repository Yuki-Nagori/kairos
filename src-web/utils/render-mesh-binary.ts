/** 渲染网格二进制解码，与 core render_mesh::encode 的 KRM1 布局对应。 */
import type { RenderMeshData } from "../types";

export function decodeRenderMesh(buffer: ArrayBuffer): RenderMeshData {
  const view = new DataView(buffer);
  if (buffer.byteLength < 16 || view.getUint32(0, true) !== 0x314d524b) {
    throw new Error("渲染网格头无效。");
  }
  const positions = view.getUint32(4, true);
  const indices = view.getUint32(8, true);
  const cells = view.getUint32(12, true);
  if (16 + 4 * (positions + indices + cells) !== buffer.byteLength) {
    throw new Error("渲染网格长度不符。");
  }
  return {
    positions: new Float32Array(buffer, 16, positions),
    indices: new Uint32Array(buffer, 16 + 4 * positions, indices),
    faceCells: new Uint32Array(buffer, 16 + 4 * (positions + indices), cells),
  };
}
