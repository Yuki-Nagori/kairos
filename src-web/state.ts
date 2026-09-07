import { createStore } from "./lib/store";
import { IpcUnavailableError } from "./lib/ipc";
import { Channel } from "@tauri-apps/api/core";
import { getSystemInfo } from "./services/system";
import {
  createProject,
  defaultCaseDir,
  listRecentProjects,
  loadProjectFile,
  saveProjectFile,
} from "./services/project";
import {
  pickExportJsonPath,
  pickOpenJsonPath,
  pickOpenProjectPath,
  pickSaveProjectPath,
} from "./services/dialog";
import {
  deleteCustomMaterial,
  exportMaterialsToFile,
  importCustomMaterials,
  listBuiltinMaterials,
  listCustomMaterials,
  upsertCustomMaterial,
} from "./services/materials";
import {
  generateVolumeMesh,
  importSampleBox,
  importStl,
  removeGeometry,
} from "./services/geometry";
import { generateOpenfoamCase } from "./services/solver";
import { pickStlPath } from "./services/dialog";
import { checkMoldNetwork } from "./services/mold";
import { cancelJob as apiCancelJob, listJobs, submitJob as apiSubmitJob } from "./services/jobs";
import type {
  GeometrySummary,
  Job,
  Material,
  MeshingReport,
  Project,
  RecentProject,
  RunnerKind,
  Study,
  SystemInfo,
} from "./types";

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
  /** 当前活跃研究（浇口 / 水路 / 工艺编辑的目标）。 */
  activeStudyId: string | null;
  /** 模具网络校验问题清单（校验按钮触发）。 */
  moldIssues: string[];
  /** 求解作业列表（调度器持有的快照）。 */
  jobs: Job[];
  /** 材料库：内置示例材料 + 用户自定义材料。 */
  materials: MaterialLibrary;
  /** 已导入的几何（摘要列表，全量网格在 Rust 会话缓存）。 */
  geometries: GeometrySummary[];
  /** 每个几何的体积网格报告（key = geometryId）。 */
  meshReports: Record<string, MeshingReport>;
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
  materials: { builtin: [], custom: [] },
  geometries: [],
  meshReports: {},
  activeStudyId: null,
  moldIssues: [],
  jobs: [],
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

/** 刷新最近项目列表（保存/打开后由内部调用）。 */
async function refreshRecents(): Promise<void> {
  try {
    const recents = await listRecentProjects();
    appStore.set({ recents });
  } catch (error) {
    setError(error);
  }
}
/** 材料库切片：内置 + 自定义（详情见 T04）。 */
interface MaterialLibrary {
  builtin: Material[];
  custom: Material[];
}

async function loadMaterials(): Promise<void> {
  const [builtin, custom] = await Promise.all([listBuiltinMaterials(), listCustomMaterials()]);
  appStore.set({ materials: { builtin, custom } });
}

/** 从 JSON 文件导入自定义材料。 */
async function importMaterialsFromPath(path: string): Promise<void> {
  appStore.set({ busy: "正在导入材料…", error: null });
  try {
    const custom = await importCustomMaterials(path);
    appStore.set({ materials: { ...appStore.get().materials, custom } });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 弹出对话框导入材料。 */
export async function importMaterials(): Promise<void> {
  const path = await pickOpenJsonPath();
  if (path) {
    await importMaterialsFromPath(path);
  }
}

/** 新增或更新一个自定义材料。 */
async function upsertMaterial(material: Material): Promise<void> {
  appStore.set({ busy: "正在保存材料…", error: null });
  try {
    const custom = await upsertCustomMaterial(material);
    appStore.set({ materials: { ...appStore.get().materials, custom } });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

export async function deleteMaterial(id: string): Promise<void> {
  appStore.set({ busy: "正在删除材料…", error: null });
  try {
    const custom = await deleteCustomMaterial(id);
    appStore.set({ materials: { ...appStore.get().materials, custom } });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 导出全部自定义材料到指定路径。 */
async function exportCustomMaterials(path: string): Promise<void> {
  const { materials } = appStore.get();
  if (materials.custom.length === 0) {
    setError("没有可导出的自定义材料。");
    return;
  }
  appStore.set({ busy: "正在导出材料…", error: null });
  try {
    await exportMaterialsToFile(path, materials.custom);
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 弹出对话框导出自定义材料。 */
export async function exportMaterials(): Promise<void> {
  const path = await pickExportJsonPath("kairos-custom-materials");
  if (path) {
    await exportCustomMaterials(path);
  }
}

/** 复制任一材料为自定义材料（新 id + 「副本」后缀）。 */
export async function copyMaterialToCustom(id: string): Promise<void> {
  const { materials } = appStore.get();
  const source = [...materials.builtin, ...materials.custom].find((m) => m.id === id);
  if (!source) {
    setError("未找到要复制的材料。");
    return;
  }
  const copy: Material = {
    ...source,
    id: `custom-${Date.now()}`,
    name: `${source.name}-副本`,
    dataNote: source.dataNote.startsWith("自定义")
      ? source.dataNote
      : `自定义副本。${source.dataNote}`,
  };
  await upsertMaterial(copy);
}

/** 导入 STL：弹出文件对话框，解析检查后入列表。 */
export async function importGeometry(): Promise<void> {
  const path = await pickStlPath();
  if (!path) {
    return;
  }
  appStore.set({ busy: "正在导入几何…", error: null });
  try {
    const summary = await importStl(path);
    appStore.set({ geometries: [...appStore.get().geometries, summary] });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 从列表与会话缓存移除几何。 */
export async function removeGeometryById(geometryId: string): Promise<void> {
  try {
    await removeGeometry(geometryId);
    appStore.set({
      geometries: appStore.get().geometries.filter((g) => g.geometryId !== geometryId),
    });
  } catch (error) {
    setError(error);
  }
}

/** 为几何生成 3D 体积网格（体素 + 5-四面体保形分解）。 */
export async function generateMesh(geometryId: string, targetSize: number): Promise<void> {
  if (!(targetSize > 0)) {
    setError("目标网格尺寸必须为正数。");
    return;
  }
  appStore.set({ busy: "正在生成网格…", error: null });
  try {
    const report = await generateVolumeMesh(geometryId, targetSize);
    appStore.set({
      meshReports: { ...appStore.get().meshReports, [geometryId]: report },
    });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 选择活跃研究；传 null 取消选择。 */
export function selectStudy(studyId: string | null): void {
  appStore.set({ activeStudyId: studyId, moldIssues: [] });
}

function activeStudy(): Study | null {
  const { project, activeStudyId } = appStore.get();
  return project?.studies.find((study) => study.id === activeStudyId) ?? null;
}

function updateActiveStudy(mutate: (study: Study) => void): void {
  const { project, activeStudyId } = appStore.get();
  const study = project?.studies.find((s) => s.id === activeStudyId);
  if (!project || !study) {
    setError("请先选择一个研究。");
    return;
  }
  mutate(study);
  appStore.set({
    project: { ...project, updatedMs: Date.now() },
  });
}

let elementSeq = 0;

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
  const study = activeStudy();
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

/** 提交求解作业（case 目录 + 核数），日志行经 Channel 由作业面板展示。 */
export async function submitJobAction(caseDir: string, cores: number): Promise<void> {
  appStore.set({ busy: "正在提交作业…", error: null });
  const channel = new Channel<string>();
  channel.onmessage = (line) => {
    if (!line.startsWith("__TIME__")) {
      // 日志行走作业面板展示；时间标记由调度器解析。
      void line;
    }
  };
  try {
    await apiSubmitJob(caseDir, cores, appStore.get().activeStudyId, channel);
    await refreshJobs();
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

export async function cancelJobAction(jobId: string): Promise<void> {
  try {
    await apiCancelJob(jobId);
    await refreshJobs();
  } catch (error) {
    setError(error);
  }
}

export async function refreshJobs(): Promise<void> {
  try {
    const jobs = await listJobs();
    appStore.set({ jobs });
  } catch (error) {
    setError(error);
  }
}

/** 导入内置样例立方体（首次使用引导）。 */
export async function importSampleGeometry(size = 10): Promise<void> {
  appStore.set({ busy: "正在导入样例…", error: null });
  try {
    const summary = await importSampleBox(size);
    appStore.set({ geometries: [...appStore.get().geometries, summary] });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 把材料登记到活跃研究。 */
export function assignMaterial(materialId: string): void {
  const { project, activeStudyId, materials } = appStore.get();
  if (!project || !activeStudyId) {
    setError("请先创建或选择一个研究。");
    return;
  }
  const exists = [...materials.builtin, ...materials.custom].some((m) => m.id === materialId);
  if (!exists) {
    setError("材料不存在。");
    return;
  }
  const studies = project.studies.map((study) =>
    study.id === activeStudyId ? { ...study, materialId } : study,
  );
  appStore.set({ project: { ...project, studies, updatedMs: Date.now() } });
}

/** 端到端提交：case 生成 → 作业入队（前置检查见 T11 流水线面板）。 */
export async function submitPipeline(cores: number): Promise<void> {
  const state = appStore.get();
  const geometry = state.geometries[0] ?? null;
  const study = state.project?.studies.find((s) => s.id === state.activeStudyId) ?? null;
  const material = study?.materialId
    ? ([...state.materials.builtin, ...state.materials.custom].find(
        (m) => m.id === study.materialId,
      ) ?? null)
    : null;
  const meshed = geometry !== null && state.meshReports[geometry.geometryId] !== undefined;

  if (geometry === null) {
    setError("请先导入几何。");
    return;
  }
  if (!meshed) {
    setError("请先生成体积网格。");
    return;
  }
  if (material === null) {
    setError("请先在研究上登记材料。");
    return;
  }
  if (study?.process == null) {
    setError("请先设置工艺并应用到研究。");
    return;
  }

  appStore.set({ busy: "正在准备求解…", error: null });
  try {
    const caseDir = await defaultCaseDir(study.id);
    await generateOpenfoamCase({
      geometryId: geometry.geometryId,
      caseDir,
      material,
      process: study.process,
      stage: "fill",
      cores,
    });
    await submitJobAction(caseDir, cores);
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}
