/** 运行时依赖 IPC：依赖清单、官方指引页与组件更新检查。 */
import { invokeCommand } from "../utils/ipc";
import type { DependencyStatus, UpdateCheck } from "../types";

/** 运行时依赖清单（目录 + 就绪探测）。 */
export function listRuntimeDependencies(): Promise<DependencyStatus[]> {
  return invokeCommand("list_runtime_dependencies");
}

/** 打开组件的官方下载 / 编译指引页。 */
export function openDependencyPage(pageUrl: string): Promise<void> {
  return invokeCommand("open_dependency_page", { pageUrl });
}

/** 在线检查组件更新（仅 release 流组件；其余组件后端会拒绝）。 */
export function checkDependencyUpdate(componentId: string): Promise<UpdateCheck> {
  return invokeCommand("check_dependency_update", { componentId });
}
