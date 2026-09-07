import { invokeCommand } from "../lib/ipc";
import type { Project, RecentProject } from "../types";

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

export function listRecentProjects(): Promise<RecentProject[]> {
  return invokeCommand("list_recent_projects");
}
