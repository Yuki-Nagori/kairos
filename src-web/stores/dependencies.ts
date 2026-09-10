/** 运行时依赖状态：就绪探测结果、受管下载清单与组件级下载/更新检查的阶段状态。 */
import { defineStore } from "pinia";
import {
  checkDependencyUpdate,
  listRuntimeDependencies,
  openDependencyPage as apiOpenDependencyPage,
} from "../api/dependencies";
import { downloadComponentFile, listDownloads } from "../api/downloads";
import type {
  ComponentStageState,
  DependencyStatus,
  DownloadedEntry,
  SavedDownload,
  UpdateCheck,
} from "../types";
import { useAppStore } from "./app";

/** SavedDownload → 跨会话清单条目（下载完成时刻即落盘时刻）。 */
function toEntry(saved: SavedDownload): DownloadedEntry {
  return {
    fileName: saved.fileName,
    sizeBytes: saved.sizeBytes,
    downloadedAtMs: Date.now(),
    extractDir: saved.extractDir,
  };
}

export const useDependenciesStore = defineStore("dependencies", {
  state: () => ({
    /** 运行时依赖状态（许可分级 + 就绪探测）。 */
    dependencies: [] as DependencyStatus[],
    /** 本次会话内下载成功的记录（key = 组件 id）；跨会话恢复走 downloadedFiles。 */
    savedDownloads: {} as Record<string, SavedDownload>,
    /** 跨会话「已下载」清单（后端 manifest.json 快照）。 */
    downloadedFiles: {} as Record<string, DownloadedEntry>,
    /** 组件下载/编译流水线的阶段状态（key = 组件 id；成功后清除该条目）。 */
    componentStages: {} as Record<string, ComponentStageState>,
    /** 组件在线更新检查结果（key = 组件 id；重新下载成功后清除）。 */
    updateChecks: {} as Record<string, UpdateCheck>,
  }),
  actions: {
    /** 刷新运行时依赖就绪状态，并恢复跨会话的「已下载」清单。 */
    async refreshDependencies(): Promise<void> {
      const app = useAppStore();
      try {
        const [dependencies, downloadedFiles] = await Promise.all([
          listRuntimeDependencies(),
          listDownloads(),
        ]);
        this.dependencies = dependencies;
        this.downloadedFiles = downloadedFiles;
      } catch (error) {
        app.setError(error);
      }
    },
    /** 打开组件官方页（下载 / 编译指引）。 */
    async openDependencyPage(pageUrl: string): Promise<void> {
      const app = useAppStore();
      try {
        await apiOpenDependencyPage(pageUrl);
      } catch (error) {
        app.setError(error);
      }
    },
    /** 更新某组件的阶段状态；null 表示清除（成功收尾）。 */
    setStage(componentId: string, stage: ComponentStageState | null): void {
      const next = { ...this.componentStages };
      if (stage === null) {
        delete next[componentId];
      } else {
        next[componentId] = stage;
      }
      this.componentStages = next;
    },
    /** 在线检查组件更新（release 流组件），结果落在 updateChecks 供面板提示。 */
    async checkUpdate(componentId: string): Promise<void> {
      const app = useAppStore();
      try {
        const check = await checkDependencyUpdate(componentId);
        this.updateChecks = { ...this.updateChecks, [componentId]: check };
      } catch (error) {
        app.setError(error);
      }
    },
    /** 应用内下载：把官方单文件直链取回受管目录。
     * 下载不占用全局 busy（大文件不应阻塞其他面板操作），
     * 进度在组件的阶段状态里行内展示；失败落定在 failed 阶段并支持重试。 */
    async downloadComponent(componentId: string, url: string): Promise<void> {
      this.setStage(componentId, { stage: "downloading", percent: 0 });
      try {
        const saved = await downloadComponentFile(componentId, url, (percent) => {
          if (this.componentStages[componentId]?.stage === "downloading") {
            this.setStage(componentId, { stage: "downloading", percent });
          }
        });
        const nextChecks = { ...this.updateChecks };
        delete nextChecks[componentId];
        this.savedDownloads = { ...this.savedDownloads, [componentId]: saved };
        this.downloadedFiles = { ...this.downloadedFiles, [componentId]: toEntry(saved) };
        this.updateChecks = nextChecks;
        this.setStage(componentId, null);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        this.setStage(componentId, { stage: "failed", error: `下载失败：${message}` });
      }
    },
  },
});
