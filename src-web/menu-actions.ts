/**
 * 原生菜单动作路由：Rust 菜单项只发射动作 id（menu-action 事件），
 * 这里统一映射到 Pinia store 动作。浏览器预览没有 IPC，listen 静默失败。
 * store 在处理器内现取：模块加载早于 main.ts 安装 Pinia，不能在顶层实例化。
 */
import { listen } from "@tauri-apps/api/event";
import { cycleTheme } from "./composables/useTheme";
import { useProjectStore } from "./stores/project";
import { useResultsStore } from "./stores/results";
import { useDependenciesStore } from "./stores/dependencies";
import { useVmStore } from "./stores/vm";

const menuActions: Record<string, () => void> = {
  "file.new": () => void useProjectStore().newProject("未命名项目"),
  "file.open": () => void useProjectStore().openProject(),
  "file.save": () => void useProjectStore().saveProject(),
  "file.saveAs": () => void useProjectStore().saveProjectAs(),
  "view.theme": () => cycleTheme(),
  "analysis.checkNetwork": () => void useProjectStore().checkNetwork(),
  "results.exportCsv": () => void useResultsStore().exportFieldCsv(),
  "tools.refreshDeps": () => void useDependenciesStore().refreshDependencies(),
  // 虚拟机：面板入口展开抽屉，动作直接走 vm store
  "tools.vmPanel": () => {
    const vm = useVmStore();
    vm.showPanel();
    void vm.refreshVmStatus();
  },
  "tools.vmStart": () => void useVmStore().startVm(),
  "tools.vmShell": () => {
    const vm = useVmStore();
    vm.showPanel();
    void vm.openShell();
  },
  "tools.vmStop": () => void useVmStore().stopVm(),
};

/** 注册原生菜单事件监听（main 启动时调用一次）。 */
export function setupMenuActions(): void {
  listen<string>("menu-action", (event) => {
    menuActions[event.payload]?.();
  }).catch(() => undefined);
}
