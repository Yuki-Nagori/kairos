import { getSystemInfo } from "../services/system";
import {
  createProject,
  listRecentProjects,
  loadProjectFile,
  saveProjectFile,
} from "../services/project";
import { pickOpenProjectPath, pickSaveProjectPath } from "../services/dialog";
import { checkMoldNetwork } from "../services/mold";
import type { Project, RunnerKind, Study } from "../types";
import { appStore, setError, type AppState } from "./store";
import { loadMaterials } from "./materials";

let studySeq = 0;
let elementSeq = 0;

/** 启动时拉取应用信息与最近项目，顺带验证 IPC 链路是否通畅。 */
export async function bootstrap(): Promise<void> {
  try {
    const [info, recents] = await Promise.all([
      getSystemInfo(),
      listRecentProjects(),
      loadMaterials(),
    ]);
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

/** 刷新最近项目列表（保存/打开后由内部调用）。 */
async function refreshRecents(): Promise<void> {
  try {
    const recents = await listRecentProjects();
    appStore.set({ recents });
  } catch (error) {
    setError(error);
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
    runnerElements: [],
    coolingChannels: [],
    process: null,
    materialId: null,
  };
  const updated: Project = {
    ...project,
    studies: [...project.studies, study],
    updatedMs: Date.now(),
  };
  appStore.set({ project: updated, activeStudyId: study.id });
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

/** 选择活跃研究；传 null 取消选择。 */
export function selectStudy(studyId: string | null): void {
  appStore.set({ activeStudyId: studyId, moldIssues: [] });
}

function activeStudy(state: AppState): Study | null {
  return state.project?.studies.find((study) => study.id === state.activeStudyId) ?? null;
}

function updateActiveStudy(mutate: (study: Study) => void): void {
  const state = appStore.get();
  const study = activeStudy(state);
  if (!state.project || !study) {
    setError("请先选择一个研究。");
    return;
  }
  mutate(study);
  appStore.set({
    project: { ...state.project, updatedMs: Date.now() },
  });
}

/** 添加流道 / 浇口单元到活跃研究。 */
export function addRunnerElement(
  kind: RunnerKind,
  diameterMm: number,
  start: [number, number, number],
  end: [number, number, number],
): void {
  updateActiveStudy((study) => {
    study.runnerElements.push({
      id: `re-${Date.now()}-${++elementSeq}`,
      kind,
      diameterMm,
      start,
      end,
    });
  });
}

export function removeRunnerElement(id: string): void {
  updateActiveStudy((study) => {
    study.runnerElements = study.runnerElements.filter((element) => element.id !== id);
  });
}

/** 添加冷却水路单元到活跃研究。 */
export function addCoolingChannel(
  diameterMm: number,
  start: [number, number, number],
  end: [number, number, number],
  inletTempC: number,
): void {
  updateActiveStudy((study) => {
    study.coolingChannels.push({
      id: `cc-${Date.now()}-${++elementSeq}`,
      diameterMm,
      start,
      end,
      inletTempC,
    });
  });
}

export function removeCoolingChannel(id: string): void {
  updateActiveStudy((study) => {
    study.coolingChannels = study.coolingChannels.filter((channel) => channel.id !== id);
  });
}

/** 调用 core 校验模具网络，问题清单入状态。 */
export async function checkNetwork(): Promise<void> {
  const study = activeStudy(appStore.get());
  if (!study) {
    setError("请先选择一个研究。");
    return;
  }
  try {
    const moldIssues = await checkMoldNetwork(study.runnerElements, study.coolingChannels);
    appStore.set({ moldIssues });
  } catch (error) {
    setError(error);
  }
}
