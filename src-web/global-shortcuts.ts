/**
 * 全局键盘快捷键接线：把 SHORTCUTS 注册表中的文件类快捷键接到 store 动作。
 * 修饰键匹配在 utils/shortcuts 按平台严格判定；命令面板（⌘K/Ctrl+K）由
 * useCommandPalette 自行监听。main 启动时注册一次。
 * store 在处理器内现取：模块加载早于 main.ts 安装 Pinia，不能在顶层实例化。
 * macOS 桌面端原生菜单加速器会先消费这些按键（本监听不重复触发），
 * 浏览器预览与 Windows/Linux 无原生菜单，由本接线兜底。
 */
import { matchesShortcut, SHORTCUTS } from "./utils/shortcuts";
import { useProjectStore } from "./stores/project";

export function setupGlobalShortcuts(): void {
  window.addEventListener("keydown", (event) => {
    const project = () => useProjectStore();
    if (matchesShortcut(event, SHORTCUTS.fileSave)) {
      event.preventDefault();
      void project().saveProject();
    } else if (matchesShortcut(event, SHORTCUTS.fileSaveAs)) {
      event.preventDefault();
      void project().saveProjectAs();
    } else if (matchesShortcut(event, SHORTCUTS.fileOpen)) {
      event.preventDefault();
      void project().openProject();
    } else if (matchesShortcut(event, SHORTCUTS.fileNew)) {
      event.preventDefault();
      void project().newProject("未命名项目");
    }
  });
}
