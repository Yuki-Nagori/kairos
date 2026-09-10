/**
 * 全局键盘快捷键（CAD 习惯）：Ctrl/Cmd+S 保存、Ctrl/Cmd+O 打开、
 * Ctrl/Cmd+N 新建。main 启动时注册一次。
 * store 在处理器内现取：模块加载早于 main.ts 安装 Pinia，不能在顶层实例化。
 */
import { useProjectStore } from "./stores/project";

export function setupGlobalShortcuts(): void {
  window.addEventListener("keydown", (event) => {
    if (!(event.ctrlKey || event.metaKey)) {
      return;
    }
    const key = event.key.toLowerCase();
    if (key === "s") {
      event.preventDefault();
      void useProjectStore().saveProject();
    } else if (key === "o") {
      event.preventDefault();
      void useProjectStore().openProject();
    } else if (key === "n") {
      event.preventDefault();
      void useProjectStore().newProject("未命名项目");
    }
  });
}
