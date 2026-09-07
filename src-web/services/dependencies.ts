import { invokeCommand } from "../lib/ipc";
import type { DependencyStatus } from "../types";

/** 运行时依赖清单（目录 + 就绪探测）。 */
export function listRuntimeDependencies(): Promise<DependencyStatus[]> {
  return invokeCommand("list_runtime_dependencies");
}

/** 打开组件的官方下载 / 编译页（GPL 走引导安装）。 */
export function openDependencyPage(pageUrl: string): Promise<void> {
  return invokeCommand("open_dependency_page", { pageUrl });
}
