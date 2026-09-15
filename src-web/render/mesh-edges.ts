/** GPU 线列表的索引布局转换；保留原顶点编号，复用表面顶点缓冲。 */
export function meshEdgeIndices(triangles: Uint32Array): Uint32Array {
  const edges = new Uint32Array(triangles.length * 2);
  for (let i = 0; i < triangles.length; i += 3) {
    const a = triangles[i]!;
    const b = triangles[i + 1]!;
    const c = triangles[i + 2]!;
    edges.set([a, b, b, c, c, a], i * 2);
  }
  return edges;
}
