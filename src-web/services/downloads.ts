import { Channel } from "@tauri-apps/api/core";
import { invokeCommand } from "../lib/ipc";
import type { SavedDownload } from "../types";

/** 下载组件到受管目录（应用数据 downloads/），进度百分比经 Channel 回传。 */
export function downloadComponentFile(
  url: string,
  onProgress: (percent: number) => void,
): Promise<SavedDownload> {
  const channel = new Channel<number>();
  channel.onmessage = onProgress;
  return invokeCommand("download_file", { url, progress: channel });
}

/** 打开下载目录（系统文件管理器）。 */
export function openDownloadsDir(): Promise<string> {
  return invokeCommand("open_downloads_dir");
}

/** 获取下载目录路径。 */
export function getDownloadsDir(): Promise<string> {
  return invokeCommand("get_downloads_dir");
}
