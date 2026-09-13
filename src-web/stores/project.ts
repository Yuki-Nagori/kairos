/** 工程文档状态：项目/研究/流道与水路单元，以及最近项目与模具网络校验问题。 */
import { defineStore } from "pinia";
import { getSystemInfo } from "../api/system";
import {
  createProject,
  listRecentProjects,
  loadProjectFile,
  saveProjectFile,
} from "../api/project";
import { pickOpenProjectPath, pickSaveProjectPath } from "../api/dialog";
import { checkMoldNetwork } from "../api/mold";
import type { Project, RunnerKind, Study } from "../types";
import { useAppStore } from "./app";
import { useMaterialsStore } from "./materials";

let studySeq = 0;
let elementSeq = 0;

/** 结构编辑只改内存，落盘统一经保存/另存为动作（writeProject）。 */
export const useProjectStore = defineStore("project", {
  state: () => ({
    /** 当前打开的工程文档；null 表示尚未打开（新建/打开后才有）。 */
    project: null as Project | null,
    /** 当前工程的保存路径；null 表示尚未保存过（保存时弹出另存为）。 */
    projectPath: null as string | null,
    /** 最近打开的工程（跨会话，来自应用数据目录）。 */
    recents: [] as Awaited<ReturnType<typeof listRecentProjects>>,
    /** 当前活跃研究（浇口 / 水路 / 工艺编辑的目标）。 */
    activeStudyId: null as string | null,
    /** 模具网络校验问题清单（校验按钮触发）。 */
    moldIssues: [] as string[],
  }),
  getters: {
    /** 当前活跃研究对象（未选择或不存在时为 null）。 */
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
    /** 新建空项目（仅内存，保存时才落盘）。 */
    async newProject(name: string): Promise<void> {
      const app = useAppStore();
      await app.withBusy("正在创建项目…", async () => {
        this.project = await createProject(name);
        this.projectPath = null;
        this.syncActiveStudy();
      });
    },
    /** 弹出文件对话框选择并打开工程。 */
    async openProject(): Promise<void> {
      const path = await pickOpenProjectPath();
      if (path) {
        await this.openProjectAtPath(path);
      }
    },
    /** 打开指定路径的工程文件。 */
    async openProjectAtPath(path: string): Promise<void> {
      const app = useAppStore();
      await app.withBusy("正在打开项目…", async () => {
        this.project = await loadProjectFile(path);
        this.projectPath = path;
        this.syncActiveStudy();
      });
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
    /** 添加研究：名称校验在本地完成（与 core 的 add_study 规则一致）。 */
    addStudy(name: string): void {
      const app = useAppStore();
      const project = this.project;
      const trimmed = name.trim();
      if (!project) {
        app.setError("请先新建或打开项目。");
        return;
      }
      if (!trimmed) {
        app.setError("研究名称不能为空。");
        return;
      }
      if (project.studies.some((study) => study.name === trimmed)) {
        app.setError(`已存在同名研究：${trimmed}`);
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
    },
    /** 切换活跃研究（工程树方案层 / 命令面板的入口）。 */
    selectStudy(id: string): void {
      if (this.project?.studies.some((study) => study.id === id)) {
        this.activeStudyId = id;
      }
    },
    /** 活跃研究原地改一格式的公共尾部：不可变字段由调用方改，这里只负责
     * 触发响应式更新并盖章 updatedMs。 */
    touchActiveStudy(mutate: (study: Study) => void): void {
      const app = useAppStore();
      const study = this.activeStudy;
      if (!this.project || !study) {
        app.setError("请先选择一个研究。");
        return;
      }
      mutate(study);
      this.project = { ...this.project, updatedMs: Date.now() };
    },
    /** 添加流道 / 浇口单元到活跃研究。 */
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
    /** 添加冷却水路单元到活跃研究。 */
    addCoolingChannel(
      diameterMm: number,
      start: [number, number, number],
      end: [number, number, number],
      inletTempC: number,
    ): void {
      this.touchActiveStudy((study) => {
        study.coolingChannels.push({
          id: `cc-${Date.now()}-${++elementSeq}`,
          diameterMm,
          start,
          end,
          inletTempC,
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
        app.setError("请先选择一个研究。");
        return;
      }
      try {
        this.moldIssues = await checkMoldNetwork(study.runnerElements, study.coolingChannels);
      } catch (error) {
        app.setError(error);
      }
    },
  },
});
