/**
 * 桌面外壳适配入口：组件经此取得能力描述（标题栏形态 / 菜单承载 /
 * 窗口控制 / 快捷键提示），不感知平台与运行时细节。
 * 能力在会话内静态（平台与 Tauri 运行时不会中途变化），无需响应式。
 */
import { currentPlatform, isTauriRuntime } from "../utils/environment";
import { resolveShellCapabilities, type ShellCapabilities } from "../utils/shell";

export function useShell(): ShellCapabilities {
  return resolveShellCapabilities(currentPlatform(), isTauriRuntime());
}
