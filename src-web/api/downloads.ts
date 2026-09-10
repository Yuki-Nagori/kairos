/** 受管下载 IPC：单文件直链下载（进度经 Channel 回传）与跨会话已下载清单。 */
import { Channel } from "@tauri-apps/api/core";
import { invokeCommand } from "../utils/ipc";
import type { DownloadedEntry, SavedDownload } from "../types";

/** 下载组件到受管目录（应用数据 downloads/），进度百分比经 Channel 回传；
 * componentId 作为 manifest.json 的键用于跨会话恢复「已下载」状态。 */
export function downloadComponentFile(
  componentId: string,
  url: string,
  onProgress: (percent: number) => void,
): Promise<SavedDownload> {
  const channel = new Channel<number>();
  channel.onmessage = onProgress;
  return invokeCommand("download_file", { componentId, url, progress: channel });
}

/** 跨会话的已下载清单（manifest.json）。 */
export function listDownloads(): Promise<Record<string, DownloadedEntry>> {
  return invokeCommand("list_downloads");
}

/** 打开下载目录（系统文件管理器）。 */
export function openDownloadsDir(): Promise<string> {
  return invokeCommand("open_downloads_dir");
}

/** 获取下载目录路径。 */
export function getDownloadsDir(): Promise<string> {
  return invokeCommand("get_downloads_dir");
}
