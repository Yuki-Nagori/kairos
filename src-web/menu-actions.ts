/**
 * 原生菜单动作路由：Rust 菜单项只发射动作 id（menu-action 事件），
 * 这里统一映射到 Pinia store 动作。浏览器预览没有 IPC，listen 静默失败。
 * store 在处理器内现取：模块加载早于 main.ts 安装 Pinia，不能在顶层实例化。
 */
import { listen } from "@tauri-apps/api/event";
import { cycleTheme } from "./composables/useTheme";
import { useAppStore } from "./stores/app";
import { useProjectStore } from "./stores/project";
import { useResultsStore } from "./stores/results";
import { useDependenciesStore } from "./stores/dependencies";
import { useVmStore } from "./stores/vm";
import { useAboutDialog } from "./components/menu-bar/useAboutDialog";

const menuActions: Record<string, () => void> = {
  "file.new": () => void useProjectStore().newProject("未命名项目"),
  "file.open": () => void useProjectStore().openProject(),
  "file.save": () => void useProjectStore().saveProject(),
  "file.saveAs": () => void useProjectStore().saveProjectAs(),
  "view.theme": () => cycleTheme(),
  "analysis.checkNetwork": () => void useProjectStore().checkNetwork(),
  "results.exportCsv": () => void useResultsStore().exportFieldCsv(),
  "tools.refreshDeps": () => void useDependenciesStore().refreshDependencies(),
  // 报告：直达报告工作台
  "report.open": () => (useAppStore().stage = "report"),
  // 关于：Windows/Linux 与命令面板弹 web 对话框（macOS 应用菜单走系统面板不经此）
  "app.about": () => useAboutDialog().showAbout(),
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
    // 载荷来自 Rust 菜单的字符串 id：只在命中注册表时触发，其余静默忽略。
    const id = event.payload as MenuActionId;
    if (id in menuActions) {
      menuActions[id]?.();
    }
  }).catch(() => undefined);
}

/** 动作 id 的编译期类型：菜单结构（useCommandRegistry）引用 id 时受此约束。 */
type MenuActionId = keyof typeof menuActions;

/** 按动作 id 直接触发：窗口内菜单栏 / 命令面板与原生菜单共用同一动作集。 */
export function runMenuAction(id: MenuActionId): void {
  menuActions[id]?.();
}
