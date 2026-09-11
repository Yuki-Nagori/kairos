/**
 * 窗口控制（Windows/Linux 无边框标题栏）：最小化 / 最大化切换 / 关闭，
 * 全部走 Tauri 窗口 API（与系统按钮同一能力）；浏览器预览里按钮不渲染。
 */
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauriRuntime } from "../../utils/environment";

export function useWindowControls() {
  /** 浏览器预览无 Tauri 运行时，窗口控制按钮不渲染。 */
  const available = isTauriRuntime();

  function minimize(): void {
    if (!available) {
      return;
    }
    void getCurrentWindow().minimize();
  }

  /** 最大化 / 还原切换（拖拽区双击的同款行为由 Tauri 核心处理）。 */
  function toggleMaximize(): void {
    if (!available) {
      return;
    }
    void getCurrentWindow().toggleMaximize();
  }

  function close(): void {
    if (!available) {
      return;
    }
    void getCurrentWindow().close();
  }

  return { available, minimize, toggleMaximize, close };
}
