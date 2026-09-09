import {
  listRuntimeDependencies,
  openDependencyPage as apiOpenDependencyPage,
} from "../services/dependencies";
import { downloadComponentFile, listDownloads } from "../services/downloads";
import type { ComponentStageState, DownloadedEntry, SavedDownload } from "../types";
import { appStore, setError } from "./store";

/** 刷新运行时依赖就绪状态，并恢复跨会话的「已下载」清单。 */
export async function refreshDependencies(): Promise<void> {
  try {
    const [dependencies, downloadedFiles] = await Promise.all([
      listRuntimeDependencies(),
      listDownloads(),
    ]);
    appStore.set({ dependencies, downloadedFiles });
  } catch (error) {
    setError(error);
  }
}

/** 打开组件官方页（下载 / 编译指引）。 */
export async function openDependencyPageAction(pageUrl: string): Promise<void> {
  try {
    await apiOpenDependencyPage(pageUrl);
  } catch (error) {
    setError(error);
  }
}

/** 更新某组件的阶段状态；null 表示清除（成功收尾）。 */
function setStage(componentId: string, stage: ComponentStageState | null): void {
  const { componentStages } = appStore.get();
  const next = { ...componentStages };
  if (stage === null) {
    delete next[componentId];
  } else {
    next[componentId] = stage;
  }
  appStore.set({ componentStages: next });
}

/** 应用内下载：把官方单文件直链取回受管目录。
 * 下载不占用全局 busy（大文件不应阻塞其他面板操作），
 * 进度在组件的阶段状态里行内展示；失败落定在 failed 阶段并支持重试。 */
export async function downloadComponent(componentId: string, url: string): Promise<void> {
  setStage(componentId, { stage: "downloading", percent: 0 });
  try {
    const saved = await downloadComponentFile(componentId, url, (percent) => {
      if (appStore.get().componentStages[componentId]?.stage === "downloading") {
        setStage(componentId, { stage: "downloading", percent });
      }
    });
    appStore.set({
      savedDownloads: { ...appStore.get().savedDownloads, [componentId]: saved },
      downloadedFiles: {
        ...appStore.get().downloadedFiles,
        [componentId]: toEntry(saved),
      },
    });
    setStage(componentId, null);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    setStage(componentId, { stage: "failed", error: `下载失败：${message}` });
  }
}

function toEntry(saved: SavedDownload): DownloadedEntry {
  return {
    fileName: saved.fileName,
    sizeBytes: saved.sizeBytes,
    downloadedAtMs: Date.now(),
    extractDir: saved.extractDir,
  };
}
