import { invokeCommand } from "../lib/ipc";
import { Channel } from "@tauri-apps/api/core";
import type { DependencyStatus } from "../types";

/** 运行时依赖清单（目录 + 就绪探测）。 */
export function listRuntimeDependencies(): Promise<DependencyStatus[]> {
  return invokeCommand("list_runtime_dependencies");
}

/** 打开组件的官方下载 / 编译指引页。 */
export function openDependencyPage(pageUrl: string): Promise<void> {
  return invokeCommand("open_dependency_page", { pageUrl });
}

/** 编译已下载的源码组件（OpenFOAM 全量构建），
 * 日志行经 Channel 流式回传；完成后 promise 以结果消息 resolve。 */
export function compileDependency(
  componentId: string,
  onLog: (line: string) => void,
): Promise<string> {
  const channel = new Channel<string>();
  channel.onmessage = onLog;
  return invokeCommand("compile_dependency", { componentId, progress: channel });
}
