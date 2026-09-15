/** 工程 IPC：项目创建、保存 / 加载、最近项目与默认 case 目录。 */
import { invokeCommand } from "../utils/ipc";
import type {
  GeometryRef,
  GeometrySummary,
  MeshRefinement,
  RestoredStudyMesh,
  Project,
  RecentProject,
} from "../types";

/** 清理工程所属的后端几何与结果会话。 */
export function resetProjectSession(): Promise<void> {
  return invokeCommand("reset_project_session");
}

/** 创建空项目（仅内存，保存时才落盘）。 */
export function createProject(name: string): Promise<Project> {
  return invokeCommand("create_project", { name });
}

/** 保存工程到指定路径（原子写入，并登记最近项目）。 */
export function saveProjectFile(path: string, project: Project): Promise<void> {
  return invokeCommand("save_project_file", { path, project });
}

/** 打开工程文件（自动登记最近项目）。 */
export function loadProjectFile(path: string): Promise<Project> {
  return invokeCommand("load_project_file", { path });
}

/** 最近打开的工程列表（跨会话）。 */
export function listRecentProjects(): Promise<RecentProject[]> {
  return invokeCommand("list_recent_projects");
}

/** 默认工作区根：`<文档目录>/kairos`（新建项目对话框初值，不创建目录）。 */
export function defaultWorkspacePath(): Promise<string> {
  return invokeCommand("default_workspace_path");
}

/** 新工程路径：`<工作区根>/<工程名>/<工程名>.kairos`（顺带创建工程目录）。 */
export function projectPath(workspace: string, projectName: string): Promise<string> {
  return invokeCommand("project_path", { workspace, projectName });
}

/** 工程的工作区根（不在工作区目录中时为 null）。 */
export function workspaceRootOf(projectPath: string): Promise<string | null> {
  return invokeCommand("workspace_root_of", { projectPath });
}

/** 把导入的几何归档进工作区 `geometry/`，返回相对路径引用。 */
export function archiveWorkspaceGeometry(
  projectPath: string,
  geometryId: string,
  sourcePath: string,
): Promise<GeometryRef> {
  return invokeCommand("archive_workspace_geometry", { projectPath, geometryId, sourcePath });
}

/** 从工作区读回几何（打开工程时恢复导入态）。 */
export function loadWorkspaceGeometry(
  projectPath: string,
  geometryId: string,
  relativePath: string,
): Promise<GeometrySummary> {
  return invokeCommand("load_workspace_geometry", { projectPath, geometryId, relativePath });
}

/** 方案网格落盘 / 读回（工作区 `mesh/<方案 id>/`）。 */
export function saveStudyMesh(
  projectPath: string,
  studyId: string,
  geometryId: string,
  targetSize: number,
  refinement: MeshRefinement | undefined,
): Promise<void> {
  return invokeCommand("save_study_mesh", {
    projectPath,
    studyId,
    geometryId,
    targetSize,
    refinement,
  });
}

export function restoreStudyMesh(
  projectPath: string,
  studyId: string,
): Promise<RestoredStudyMesh | null> {
  return invokeCommand("restore_study_mesh", { projectPath, studyId });
}

/** 新方案的默认 case 目录：工作区内 `<工作区>/cases/<方案 id>/<运行 id>`，散装工程回退应用数据目录。 */
export function defaultCaseDir(studyId: string, projectPath?: string | null): Promise<string> {
  return invokeCommand("default_case_dir", { studyId, projectPath: projectPath ?? null });
}

/** 把报告写进工作区 `reports/`，返回写入路径（散装工程会校验失败）。 */
export function saveReportToWorkspace(
  projectPath: string,
  fileName: string,
  content: string,
): Promise<string> {
  return invokeCommand("save_report_to_workspace", { projectPath, fileName, content });
}

/** 报告导出 PPTX 的幻灯片载荷（与 Rust `ReportSlidePayload` 镜像）。 */
export interface ReportSlidePayload {
  title: string;
  bullets: string[];
  /** 快照 dataURL（PNG / JPEG），尺寸由导出侧从字节读取。 */
  images?: string[];
}

/** 报告导出为 PPTX（core 生成、命令层落盘到工作区 reports/，返回写入路径）。 */
export function saveReportPptxToWorkspace(
  projectPath: string,
  fileName: string,
  title: string,
  slides: ReportSlidePayload[],
): Promise<string> {
  return invokeCommand("save_report_pptx_to_workspace", {
    projectPath,
    fileName,
    title,
    slides,
  });
}
