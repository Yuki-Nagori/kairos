import { invokeCommand } from "../lib/ipc";
import type { GeometrySummary, MeshingReport } from "../types";

/** 导入 STL（全量网格留在 Rust 侧），返回摘要与健康检查结果。 */
export function importStl(path: string): Promise<GeometrySummary> {
  return invokeCommand("import_stl", { path });
}

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
