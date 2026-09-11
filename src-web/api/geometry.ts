/** 几何 IPC：STL 导入、体积网格生成（内置与 Gmsh 引擎）、双域网格与渲染网格导出。 */
import { invokeCommand } from "../utils/ipc";
import type {
  DualDomainReport,
  GeometrySummary,
  MeshingReport,
  MeshRefinement,
  MidplaneReport,
  RepairOutcome,
  RenderMeshData,
  RunnerElement,
} from "../types";

/** 导入 STL（全量网格留在 Rust 侧），返回摘要与健康检查结果。 */
export function importStl(path: string): Promise<GeometrySummary> {
  return invokeCommand("import_stl", { path });
}

/** 移除几何并释放其会话缓存。 */
export function removeGeometry(geometryId: string): Promise<void> {
  return invokeCommand("remove_geometry", { geometryId });
}

/** 对已导入几何生成 3D 体积网格，返回统计报告。refinement 仅体素引擎支持。 */
export function generateVolumeMesh(
  geometryId: string,
  targetSize: number,
  refinement?: MeshRefinement,
): Promise<MeshingReport> {
  return invokeCommand("generate_volume_mesh", { geometryId, targetSize, refinement });
}

/** 导入内置样例立方体（首次使用引导）。 */
export function importSampleBox(size: number): Promise<GeometrySummary> {
  return invokeCommand("import_sample_box", { size });
}

/** 导入 STEP 镶嵌网格（AP242 子集）。 */
export function importStep(path: string): Promise<GeometrySummary> {
  return invokeCommand("import_step", { path });
}

/** 导入 IGES 镶嵌网格（实体 106 / 63 子集）。 */
export function importIges(path: string): Promise<GeometrySummary> {
  return invokeCommand("import_iges", { path });
}

/** 修复几何（顶点焊接 / 孔洞填充 / 法向一致化），返回更新后的摘要与修复报告。 */
export function repairGeometry(geometryId: string): Promise<RepairOutcome> {
  return invokeCommand("repair_geometry", { geometryId });
}

/** 导出视口渲染网格（体积边界面或 STL 表面）。 */
export function getRenderMesh(geometryId: string): Promise<RenderMeshData> {
  return invokeCommand("get_render_mesh", { geometryId });
}

/** 生成 Gmsh 引擎体积网格（需已下载 Gmsh 并定位到可执行文件）。 */
export function generateGmshMesh(geometryId: string, targetSize: number): Promise<MeshingReport> {
  return invokeCommand("generate_gmsh_mesh", { geometryId, targetSize });
}

/** 生成双域网格：表面厚度配对 + 杆系（流道/浇口）梁单元耦合。 */
export function generateDualDomainMesh(
  geometryId: string,
  runners: RunnerElement[],
): Promise<DualDomainReport> {
  return invokeCommand("generate_dual_domain_mesh", { geometryId, runners });
}

/** 生成中面网格：顶点配对法（1D/2.5D 快速分析路线），杆系梁耦合中面节点。 */
export function generateMidplaneMesh(
  geometryId: string,
  runners: RunnerElement[],
): Promise<MidplaneReport> {
  return invokeCommand("generate_midplane_mesh", { geometryId, runners });
}
