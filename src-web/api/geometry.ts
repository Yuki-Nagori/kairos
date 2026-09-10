/** 几何 IPC：STL 导入、体积网格生成（内置与 Gmsh 引擎）与渲染网格导出。 */
import { invokeCommand } from "../utils/ipc";
import type { GeometrySummary, MeshingReport } from "../types";

/** 导入 STL（全量网格留在 Rust 侧），返回摘要与健康检查结果。 */
export function importStl(path: string): Promise<GeometrySummary> {
  return invokeCommand("import_stl", { path });
}

/** 移除几何并释放其会话缓存。 */
export function removeGeometry(geometryId: string): Promise<void> {
  return invokeCommand("remove_geometry", { geometryId });
}

/** 对已导入几何生成 3D 体积网格，返回统计报告。 */
export function generateVolumeMesh(geometryId: string, targetSize: number): Promise<MeshingReport> {
  return invokeCommand("generate_volume_mesh", { geometryId, targetSize });
}

/** 导入内置样例立方体（首次使用引导）。 */
export function importSampleBox(size: number): Promise<GeometrySummary> {
  return invokeCommand("import_sample_box", { size });
}

interface RenderMeshData {
  positions: number[];
  indices: number[];
  faceCells: number[];
}

/** 导出视口渲染网格（体积边界面或 STL 表面）。 */
export function getRenderMesh(geometryId: string): Promise<RenderMeshData> {
  return invokeCommand("get_render_mesh", { geometryId });
}

/** 生成 Gmsh 引擎体积网格（需已下载 Gmsh 并定位到可执行文件）。 */
export function generateGmshMesh(geometryId: string, targetSize: number): Promise<MeshingReport> {
  return invokeCommand("generate_gmsh_mesh", { geometryId, targetSize });
}
