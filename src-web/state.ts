import { createStore } from "./lib/store";
import { IpcUnavailableError } from "./lib/ipc";
import { getSystemInfo } from "./services/system";
import {
  createProject,
  listRecentProjects,
  loadProjectFile,
  saveProjectFile,
} from "./services/project";
import { pickOpenProjectPath, pickSaveProjectPath } from "./services/dialog";
import type { Project, RecentProject, Study, SystemInfo } from "./types";

/** 全局应用状态：组件经 select/subscribe 订阅，只能通过本文件的动作函数修改。 */
interface AppState {
  /** 应用信息，bootstrap 成功后填充；非 null 即代表 IPC 链路通畅。 */
  info: SystemInfo | null;
  /** 当前打开的工程文档；null 表示尚未打开（新建/打开后才有）。 */
  project: Project | null;
  /** 当前工程的保存路径；null 表示尚未保存过（保存时弹出另存为）。 */
  projectPath: string | null;
  /** 最近打开的工程（跨会话，来自应用数据目录）。 */
  recents: RecentProject[];
  /** 进行中的异步操作提示文案，标题栏展示；null 表示空闲。 */
  busy: string | null;
  /** 最近一次错误；info 为环境提示（浏览器预览，自动消失），否则是真实失败。 */
  error: { message: string; info: boolean } | null;
}

export const initialAppState: AppState = {
  info: null,
  project: null,
  projectPath: null,
  recents: [],
  busy: null,
  error: null,
};

export const appStore = createStore<AppState>(initialAppState);

let errorTimer: ReturnType<typeof setTimeout> | undefined;
let studySeq = 0;

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** 统一的错误入口：IPC 不可用是预期内的环境提示（自动消失），其余才是真实失败。 */
export function setError(error: unknown): void {
  const message = toMessage(error);
  const info = error instanceof IpcUnavailableError;
  if (errorTimer !== undefined) {
    clearTimeout(errorTimer);
  }
  appStore.set({ error: { message, info } });
  if (info) {
    errorTimer = setTimeout(() => {
      if (appStore.get().error?.message === message) {
        appStore.set({ error: null });
      }
    }, 8000);
  }
}

/** 启动时拉取应用信息与最近项目，顺带验证 IPC 链路是否通畅。 */
export async function bootstrap(): Promise<void> {
  try {
    const [info, recents] = await Promise.all([getSystemInfo(), listRecentProjects()]);
    appStore.set({ info, recents });
  } catch (error) {
    setError(error);
  }
}

/** 新建空项目（仅内存，保存时才落盘）。 */
export async function newProject(name: string): Promise<void> {
  appStore.set({ busy: "正在创建项目…", error: null });
  try {
    const project = await createProject(name);
    appStore.set({ project, projectPath: null });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 打开指定路径的工程文件。 */
export async function openProjectAtPath(path: string): Promise<void> {
  appStore.set({ busy: "正在打开项目…", error: null });
  try {
    const project = await loadProjectFile(path);
    appStore.set({ project, projectPath: path });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 弹出文件对话框选择并打开工程。 */
export async function openProject(): Promise<void> {
  const path = await pickOpenProjectPath();
  if (path) {
    await openProjectAtPath(path);
  }
}

/** 保存工程：已有路径直接保存，否则走另存为。 */
export async function saveProject(): Promise<void> {
  const { project, projectPath } = appStore.get();
  if (!project) {
    return;
  }
  const path = projectPath ?? (await pickSaveProjectPath(project.name));
  if (!path) {
    return;
  }
  appStore.set({ busy: "正在保存项目…", error: null });
  try {
    await saveProjectFile(path, project);
    appStore.set({ projectPath: path });
    await refreshRecents();
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 另存为：强制弹出路径选择。 */
export async function saveProjectAs(): Promise<void> {
  const { project } = appStore.get();
  if (!project) {
    return;
  }
  const path = await pickSaveProjectPath(project.name);
  if (!path) {
    return;
  }
  appStore.set({ busy: "正在保存项目…", error: null });
  try {
    await saveProjectFile(path, project);
    appStore.set({ projectPath: path });
    await refreshRecents();
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 添加研究：名称校验在本地完成（与 core 的 add_study 规则一致）。 */
export function addStudy(name: string): void {
  const { project } = appStore.get();
  const trimmed = name.trim();
  if (!project) {
    setError("请先新建或打开项目。");
    return;
  }
  if (!trimmed) {
    setError("研究名称不能为空。");
    return;
  }
  if (project.studies.some((study) => study.name === trimmed)) {
    setError(`已存在同名研究：${trimmed}`);
    return;
  }
  const study: Study = {
    id: `study-${Date.now()}-${++studySeq}`,
    name: trimmed,
    createdMs: Date.now(),
  };
  const updated: Project = {
    ...project,
    studies: [...project.studies, study],
    updatedMs: Date.now(),
  };
  appStore.set({ project: updated });
}

export function removeStudy(studyId: string): void {
  const { project } = appStore.get();
  if (!project || !project.studies.some((study) => study.id === studyId)) {
    return;
  }
  appStore.set({
    project: {
      ...project,
      studies: project.studies.filter((study) => study.id !== studyId),
      updatedMs: Date.now(),
    },
  });
}

/** 刷新最近项目列表（保存/打开后由内部调用）。 */
async function refreshRecents(): Promise<void> {
  try {
    const recents = await listRecentProjects();
    appStore.set({ recents });
  } catch (error) {
    setError(error);
  }
}
