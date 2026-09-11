/** 确定性基准网格资产（T39 验收协议）：由三角数反推方格分辨率，
 * 顶点坐标与拓扑完全由公式决定——同一 triangleCount 两次生成逐字节一致。 */

interface BenchmarkMesh {
  positions: Float32Array;
  indices: Uint32Array;
  faceCells: Uint32Array;
}

/** 生成 triangleCount 个三角形的平面网格（2 × n² 四象限）。
 * triangleCount 取偶数，n = sqrt(triangleCount / 2)。 */
export function deterministicGridMesh(triangleCount: number): BenchmarkMesh {
  if (triangleCount <= 0 || triangleCount % 2 !== 0) {
    throw new Error("triangleCount 必须为正偶数。");
  }
  const n = Math.round(Math.sqrt(triangleCount / 2));
  if (n * n * 2 !== triangleCount) {
    throw new Error(`triangleCount = ${triangleCount} 无法表示为 2 × n² 方格网格。`);
  }
  const vertexCount = (n + 1) * (n + 1);
  const positions = new Float32Array(vertexCount * 3);
  const step = 1 / n;
  for (let row = 0; row <= n; row += 1) {
    for (let column = 0; column <= n; column += 1) {
      const vertex = row * (n + 1) + column;
      positions[vertex * 3] = column * step;
      positions[vertex * 3 + 1] = row * step;
      positions[vertex * 3 + 2] = 0;
    }
  }
  const indices = new Uint32Array(triangleCount * 3);
  const faceCells = new Uint32Array(triangleCount);
  let face = 0;
  for (let row = 0; row < n; row += 1) {
    for (let column = 0; column < n; column += 1) {
      const v00 = row * (n + 1) + column;
      const v10 = v00 + 1;
      const v01 = v00 + (n + 1);
      const v11 = v01 + 1;
      indices[face * 3] = v00;
      indices[face * 3 + 1] = v10;
      indices[face * 3 + 2] = v11;
      faceCells[face] = face;
      face += 1;
      indices[face * 3] = v00;
      indices[face * 3 + 1] = v11;
      indices[face * 3 + 2] = v01;
      faceCells[face] = face;
      face += 1;
    }
  }
  return { positions, indices, faceCells };
}
