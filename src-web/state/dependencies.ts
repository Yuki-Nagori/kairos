import {
  listRuntimeDependencies,
  openDependencyPage as apiOpenDependencyPage,
} from "../services/dependencies";
import { downloadComponentFile, listDownloads } from "../services/downloads";
import type { DownloadedEntry, SavedDownload } from "../types";
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

/** 清除某组件的失败标记（重试前调用）。 */
function clearDownloadFailure(componentId: string): void {
  const { downloadErrors } = appStore.get();
  if (componentId in downloadErrors) {
    const next = { ...downloadErrors };
    delete next[componentId];
    appStore.set({ downloadErrors: next });
  }
}

/** 应用内下载：把官方单文件直链取回受管目录。
 * 下载不占用全局 busy（大文件不应阻塞其他面板操作），
 * 进度行内展示；失败信息落在行内并支持重试。 */
export async function downloadComponent(componentId: string, url: string): Promise<void> {
  clearDownloadFailure(componentId);
  appStore.set({ downloadProgress: { ...appStore.get().downloadProgress, [componentId]: 0 } });
  try {
    const saved = await downloadComponentFile(componentId, url, (percent) => {
      appStore.set({
        downloadProgress: { ...appStore.get().downloadProgress, [componentId]: percent },
      });
    });
    appStore.set({
      savedDownloads: { ...appStore.get().savedDownloads, [componentId]: saved },
      downloadedFiles: {
        ...appStore.get().downloadedFiles,
        [componentId]: toEntry(saved),
      },
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    appStore.set({ downloadErrors: { ...appStore.get().downloadErrors, [componentId]: message } });
  } finally {
    const { downloadProgress } = appStore.get();
    if (componentId in downloadProgress) {
      const next = { ...downloadProgress };
      delete next[componentId];
      appStore.set({ downloadProgress: next });
    }
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
