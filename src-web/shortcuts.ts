/**
 * 全局键盘快捷键（CAD 习惯）：Ctrl/Cmd+S 保存、Ctrl/Cmd+O 打开、
 * Ctrl/Cmd+N 新建。main 启动时注册一次。
 */
import { newProject, openProject, saveProject } from "./state";

export function setupGlobalShortcuts(): void {
  window.addEventListener("keydown", (event) => {
    if (!(event.ctrlKey || event.metaKey)) {
      return;
    }
    const key = event.key.toLowerCase();
    if (key === "s") {
      event.preventDefault();
      void saveProject();
    } else if (key === "o") {
      event.preventDefault();
      void openProject();
    } else if (key === "n") {
      event.preventDefault();
      void newProject("未命名项目");
    }
  });
}
