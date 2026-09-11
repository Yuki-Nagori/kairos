/**
 * 标题栏自绘窗口控制（Windows/Linux 无边框窗口）：最小化 / 最大化切换 / 关闭。
 * 可用性由外壳适配层给出——不可用时动作整体降级为空操作，调用方只管绑定。
 */
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useShell } from "../../composables/useShell";

export function useWindowControls() {
  const available = useShell().windowControls;
  // 仅在可用（Tauri 运行时）时取窗口句柄，浏览器预览不触达 Tauri API。
  const api = available ? getCurrentWindow() : null;

  return {
    available,
    minimize: (): void => void api?.minimize(),
    toggleMaximize: (): void => void api?.toggleMaximize(),
    close: (): void => void api?.close(),
  };
}
