/** 工程文档状态：项目/方案/流道与水路单元，以及最近项目与模具网络校验问题。 */
import { defineStore } from "pinia";
import { getSystemInfo } from "../api/system";
import {
  createProject,
  resetProjectSession,
  defaultWorkspacePath,
  listRecentProjects,
  loadProjectFile,
  projectPath,
  saveProjectFile,
  saveReportPptxToWorkspace,
  saveReportToWorkspace,
  workspaceRootOf,
  type ReportSlidePayload,
} from "../api/project";
import { pickOpenProjectPath, pickSaveProjectPath, pickWorkspaceDir } from "../api/dialog";
import { checkMoldNetwork } from "../api/mold";
import type { Project, RunnerKind, Study, GeometryRef } from "../types";
import { useDebounceFn } from "@vueuse/core";
import { useAppStore } from "./app";
import { useMaterialsStore } from "./materials";
import { useGeometryStore } from "./geometry";
import { useResultsStore } from "./results";
import { useProcessStore } from "./process";
import { useViewportStore } from "./viewport";

let studySeq = 0;
let elementSeq = 0;

/** 方案配置（材料 / 工艺 / 杆系）与几何引用编辑后防抖自动保存：
 *  这些编辑不落盘的话，关掉再打开工程就全丢了。显式保存 / 另存为仍走 writeProject。 */
const AUTO_SAVE_DELAY_MS = 800;
/** 冷却介质默认口径：水 0.05 kg/s、cp 4180 J/kg/K（面板初值与 core 默认一致）。 */
export const DEFAULT_COOLANT_MASS_FLOW_KG_S = 0.05;
export const WATER_SPECIFIC_HEAT = 4180;

/** 每个 store 独立管理定时资源，工程代数使切换前的回调失效。 */
const saveTimers = new WeakMap<object, (epoch: number) => Promise<void>>();

export const useProjectStore = defineStore("project", {
  state: () => ({
    /** 当前打开的工程文档；null 表示尚未打开（新建/打开后才有）。 */
    project: null as Project | null,
    autoSaveEpoch: 0,
    /** 当前工程的保存路径；null 表示尚未保存过（保存时弹出另存为）。 */
    projectPath: null as string | null,
    /** 最近打开的工程（跨会话，来自应用数据目录）。 */
    recents: [] as Awaited<ReturnType<typeof listRecentProjects>>,
    /** 当前活跃方案（浇口 / 水路 / 工艺编辑的目标）。 */
    activeStudyId: null as string | null,
    /** 模具网络校验问题清单（校验按钮触发）。 */
    moldIssues: [] as string[],
    /** 工作区根目录（工程位于「文档/kairos/<工程目录>/」内时非空）。 */
    workspaceRoot: null as string | null,
  }),
  getters: {
    /** 当前活跃方案对象（未选择或不存在时为 null）。 */
    activeStudy(state): Study | null {
      return state.project?.studies.find((study) => study.id === state.activeStudyId) ?? null;
    },
  },
  actions: {
    /** 启动时拉取应用信息与最近项目，顺带验证 IPC 链路是否通畅。 */
    async bootstrap(): Promise<void> {
      const app = useAppStore();
      try {
        const [info, recents] = await Promise.all([
          getSystemInfo(),
          listRecentProjects(),
          useMaterialsStore().loadMaterials(),
        ]);
        app.info = info;
        this.recents = recents;
      } catch (error) {
        app.setError(error);
      }
    },
    /** 新建项目（仅内存，保存时才落盘）；返回是否创建成功——调用方（新建对话框）
     *  据此决定是否收起，失败原因已进全局错误通道。
     *  workspace 为用户选定的工作区根（留空用默认 `<文档目录>/kairos`）。 */
    async newProject(name: string, workspace = ""): Promise<boolean> {
      const app = useAppStore();
      if (app.working) {
        app.setError("请等待当前操作完成后再切换工程。");
        return false;
      }
      const created = await app.withBusy("正在创建项目…", async () => {
        await this.flushCurrentProject();
        const project = await createProject(name);
        // 工程目录：<工作区根>/<工程名>/<工程名>.kairos（文件名与项目名一致）。
        const path = await projectPath(workspace, name);
        await saveProjectFile(path, project);
        await this.clearProjectSession();
        this.project = project;
        this.projectPath = path;
        this.workspaceRoot = await workspaceRootOf(path);
        this.syncActiveStudy();
        await this.refreshRecents();
        return true;
      });
      return created ?? false;
    },
    /** 弹出文件对话框选择并打开工程。 */
    async openProject(): Promise<void> {
      const path = await pickOpenProjectPath();
      if (path) {
        await this.openProjectAtPath(path);
      }
    },
    /** 打开指定路径的工程文件；工作区工程顺带恢复几何与网格（由调用方在
     *  打开后调用 geometry store 的 restoreWorkspaceContent）。 */
    async openProjectAtPath(path: string): Promise<void> {
      const app = useAppStore();
      if (app.working) {
        app.setError("请等待当前操作完成后再切换工程。");
        return;
      }
      await app.withBusy("正在打开项目…", async () => {
        await this.flushCurrentProject();
        const project = await loadProjectFile(path);
        await this.clearProjectSession();
        this.project = project;
        this.projectPath = path;
        this.syncActiveStudy();
        await this.refreshWorkspaceRoot();
        await useGeometryStore().restoreWorkspaceContent();
      });
    },
    /** 切换前保存旧工程；保存失败抛给切换动作，保留旧工程供重试。 */
    async flushCurrentProject(): Promise<void> {
      this.autoSaveEpoch += 1;
      if (this.project !== null && this.projectPath !== null) {
        await saveProjectFile(this.projectPath, this.project);
      }
    },
    /** 后端清理成功后再清前端，防止继续使用另一工程的几何与结果。 */
    async clearProjectSession(): Promise<void> {
      await resetProjectSession();
      useGeometryStore().$reset();
      useResultsStore().$reset();
      useProcessStore().$reset();
      useViewportStore().setMeshLoaded(false);
      this.moldIssues = [];
      this.activeStudyId = null;
    },
    /** 刷新工作区根（打开 / 保存后调用）：散装工程为 null。 */
    async refreshWorkspaceRoot(): Promise<void> {
      const path = this.projectPath;
      if (path === null) {
        this.workspaceRoot = null;
        return;
      }
      try {
        this.workspaceRoot = await workspaceRootOf(path);
      } catch {
        // 工作区判定失败不影响工程本身（按散装处理）
        this.workspaceRoot = null;
      }
    },
    /** 登记几何引用（导入归档后调用）：同 id 就地覆盖（保持列表顺序），否则追加。 */
    upsertGeometryRef(reference: GeometryRef): void {
      if (this.project === null) {
        return;
      }
      const index = this.project.geometries.findIndex((entry) => entry.id === reference.id);
      const geometries = [...this.project.geometries];
      if (index >= 0) {
        geometries[index] = reference;
      } else {
        geometries.push(reference);
      }
      this.project = { ...this.project, geometries, updatedMs: Date.now() };
      this.scheduleAutoSave();
    },
    /** 装载工程后的活跃方案兜底：原选中项不在新工程里就落到首个方案。
     * 活跃方案是材料 / 工艺 / 浇注系统的编辑目标，缺了它整条工作流无处落笔。 */
    syncActiveStudy(): void {
      const studies = this.project?.studies ?? [];
      if (!studies.some((study) => study.id === this.activeStudyId)) {
        this.activeStudyId = studies[0]?.id ?? null;
      }
    },
    /** 保存工程：已有路径直接保存，否则走另存为。 */
    async saveProject(): Promise<void> {
      if (!this.project) {
        return;
      }
      const path = this.projectPath ?? (await pickSaveProjectPath(this.project.name));
      if (!path) {
        return;
      }
      await this.writeProject(path);
    },
    /** 另存为：强制弹出路径选择。 */
    async saveProjectAs(): Promise<void> {
      if (!this.project) {
        return;
      }
      const path = await pickSaveProjectPath(this.project.name);
      if (!path) {
        return;
      }
      await this.writeProject(path);
    },
    /** 落盘 + 刷新最近项目（保存与另存为的公共尾部）。 */
    async writeProject(path: string): Promise<void> {
      const app = useAppStore();
      await app.withBusy("正在保存项目…", async () => {
        await saveProjectFile(path, this.project!);
        this.projectPath = path;
        await this.refreshRecents();
      });
    },
    /** 刷新最近项目列表（保存/打开后由内部调用）。 */
    async refreshRecents(): Promise<void> {
      const app = useAppStore();
      try {
        this.recents = await listRecentProjects();
      } catch (error) {
        app.setError(error);
      }
    },
    /** 添加方案：名称校验在本地完成（与 core 的 add_study 规则一致）。 */
    addStudy(name: string): void {
      const app = useAppStore();
      const project = this.project;
      const trimmed = name.trim();
      if (!project) {
        app.setError("请先新建或打开项目。");
        return;
      }
      if (!trimmed) {
        app.setError("方案名称不能为空。");
        return;
      }
      if (project.studies.some((study) => study.name === trimmed)) {
        app.setError(`已存在同名方案：${trimmed}`);
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
      this.project = {
        ...project,
        studies: [...project.studies, study],
        updatedMs: Date.now(),
      };
      this.activeStudyId = study.id;
      this.scheduleAutoSave();
    },
    /** 切换活跃方案（工程树方案层 / 命令面板的入口）。 */
    selectStudy(id: string): void {
      if (this.project?.studies.some((study) => study.id === id)) {
        this.activeStudyId = id;
      }
    },
    /** 活跃方案原地改一格式的公共尾部：不可变字段由调用方改，这里只负责
     * 触发响应式更新并盖章 updatedMs。 */
    touchActiveStudy(mutate: (study: Study) => void): void {
      const app = useAppStore();
      const study = this.activeStudy;
      if (!this.project || !study) {
        app.setError("请先选择一个方案。");
        return;
      }
      mutate(study);
      this.project = { ...this.project, updatedMs: Date.now() };
      this.scheduleAutoSave();
    },
    /** 派发一次防抖自动保存（无工程 / 无路径时不排队：散装工程仍只在显式保存时落盘）。 */
    scheduleAutoSave(): void {
      if (this.project === null || this.projectPath === null) {
        return;
      }
      void this.debouncedAutoSave();
    },
    /** 防抖后的自动保存入口（`useDebounceFn` 管定时器，语义与手写一致：
     *  连续编辑只落盘一次；显式保存仍走 writeProject 立即落盘）。 */
    debouncedAutoSave(): Promise<void> {
      let save = saveTimers.get(this);
      if (save === undefined) {
        save = useDebounceFn(async (epoch: number) => {
          if (this.autoSaveEpoch === epoch) {
            await this.autoSaveNow();
          }
        }, AUTO_SAVE_DELAY_MS);
        saveTimers.set(this, save);
      }
      return save(this.autoSaveEpoch);
    },
    /** 立即落盘当前工程（自动保存尾部）；失败进全局错误，不打断编辑。 */
    async autoSaveNow(): Promise<void> {
      const path = this.projectPath;
      const project = this.project;
      if (path === null || project === null) {
        return;
      }
      try {
        await saveProjectFile(path, project);
      } catch (error) {
        useAppStore().setError(error);
      }
    },
    /** 添加流道 / 浇口单元到活跃方案。 */
    addRunnerElement(
      kind: RunnerKind,
      diameterMm: number,
      start: [number, number, number],
      end: [number, number, number],
    ): void {
      this.touchActiveStudy((study) => {
        study.runnerElements.push({
          id: `re-${Date.now()}-${++elementSeq}`,
          kind,
          diameterMm,
          start,
          end,
        });
      });
    },
    removeRunnerElement(id: string): void {
      this.touchActiveStudy((study) => {
        study.runnerElements = study.runnerElements.filter((element) => element.id !== id);
      });
    },
    /** 添加冷却水路单元到活跃方案。 */
    addCoolingChannel(
      diameterMm: number,
      start: [number, number, number],
      end: [number, number, number],
      inletTempC: number,
      massFlowRateKgS = DEFAULT_COOLANT_MASS_FLOW_KG_S,
      specificHeatJKgK = WATER_SPECIFIC_HEAT,
    ): void {
      this.touchActiveStudy((study) => {
        study.coolingChannels.push({
          id: `cc-${Date.now()}-${++elementSeq}`,
          diameterMm,
          start,
          end,
          inletTempC,
          massFlowRateKgS,
          specificHeatJKgK,
        });
      });
    },
    removeCoolingChannel(id: string): void {
      this.touchActiveStudy((study) => {
        study.coolingChannels = study.coolingChannels.filter((channel) => channel.id !== id);
      });
    },
    /** 调用 core 校验模具网络，问题清单入状态。 */
    async checkNetwork(): Promise<void> {
      const app = useAppStore();
      const study = this.activeStudy;
      if (!study) {
        app.setError("请先选择一个方案。");
        return;
      }
      try {
        this.moldIssues = await checkMoldNetwork(study.runnerElements, study.coolingChannels);
      } catch (error) {
        app.setError(error);
      }
    },
    /** 平台默认工作区根（`<文档目录>/kairos`）；取不到返回 null。
     *  浏览器预览等 IPC 不可用属预期内场景：对话框保留上次值让用户手填，
     *  不报错（否则一开对话框就弹环境提示）。 */
    async defaultWorkspace(): Promise<string | null> {
      try {
        return await defaultWorkspacePath();
      } catch {
        return null;
      }
    },
    /** 用系统目录选择器改工作区落点（取消返回 null）。 */
    async chooseWorkspaceDir(initial: string): Promise<string | null> {
      try {
        return await pickWorkspaceDir(initial);
      } catch (error) {
        useAppStore().setError(error);
        return null;
      }
    },
    /** 报告 HTML 写入工作区 reports/：返回落盘路径；没有工作区或写入失败返回 null
     *  （失败原因进全局错误）。调用方据此回退浏览器下载——无论工作区是否可写，
     *  报告都必须拿得到。 */
    async saveReport(fileName: string, html: string): Promise<string | null> {
      const projectPath = this.projectPath;
      if (projectPath === null || this.workspaceRoot === null) {
        return null;
      }
      try {
        return await saveReportToWorkspace(projectPath, fileName, html);
      } catch (error) {
        useAppStore().setError(error);
        return null;
      }
    },
    /** 报告 PPTX 写入工作区 reports/：返回落盘路径；散装工程 / 写入失败返回 null。 */
    async saveReportPptx(title: string, slides: ReportSlidePayload[]): Promise<string | null> {
      const projectPath = this.projectPath;
      const project = this.project;
      if (projectPath === null || project === null) {
        useAppStore().setError("散装工程请先保存到工作区，再导出 PPTX。");
        return null;
      }
      try {
        return await saveReportPptxToWorkspace(projectPath, `${project.name}-报告`, title, slides);
      } catch (error) {
        useAppStore().setError(error);
        return null;
      }
    },
  },
});
