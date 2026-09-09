/**
 * 原生菜单动作路由：Rust 菜单项只发射动作 id（menu-action 事件），
 * 这里统一映射到状态分片动作。浏览器预览没有 IPC，listen 静默失败。
 */
import { listen } from "@tauri-apps/api/event";
import { cycleTheme } from "./theme";
import {
  checkNetwork,
  exportFieldCsv,
  newProject,
  openProject,
  openVmShellAction,
  refreshDependencies,
  refreshVmStatus,
  saveProject,
  saveProjectAs,
  showVmPanel,
  startVmAction,
  stopVmAction,
} from "./state";

const menuActions: Record<string, () => void> = {
  "file.new": () => void newProject("未命名项目"),
  "file.open": () => void openProject(),
  "file.save": () => void saveProject(),
  "file.saveAs": () => void saveProjectAs(),
  "view.theme": () => cycleTheme(),
  "analysis.checkNetwork": () => void checkNetwork(),
  "results.exportCsv": () => void exportFieldCsv(),
  "tools.refreshDeps": () => void refreshDependencies(),
  // 虚拟机：面板入口展开抽屉，动作直接走状态分片
  "tools.vmPanel": () => {
    showVmPanel();
    void refreshVmStatus();
  },
  "tools.vmStart": () => void startVmAction(),
  "tools.vmShell": () => {
    showVmPanel();
    void openVmShellAction();
  },
  "tools.vmStop": () => void stopVmAction(),
};

/** 注册原生菜单事件监听（main 启动时调用一次）。 */
export function setupMenuActions(): void {
  listen<string>("menu-action", (event) => {
    menuActions[event.payload]?.();
  }).catch(() => undefined);
}
