import { invokeCommand } from "../lib/ipc";
import type { GeometrySummary } from "../types";

/** 导入 STL（全量网格留在 Rust 侧），返回摘要与健康检查结果。 */
export function importStl(path: string): Promise<GeometrySummary> {
  return invokeCommand("import_stl", { path });
}

export function removeGeometry(geometryId: string): Promise<void> {
  return invokeCommand("remove_geometry", { geometryId });
}
