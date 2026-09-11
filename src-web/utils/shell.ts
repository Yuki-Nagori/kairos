/**
 * 桌面外壳适配层：把「平台 × 运行时」差异收敛为一组能力描述。
 * 组件按能力渲染（标题栏形态 / 菜单承载 / 窗口控制），不写平台分支；
 * 平台判定与快捷键文案复用 environment / shortcuts，此处只做一次性解析。
 */
import type { Platform } from "./environment";
import { shortcutLabel, SHORTCUTS } from "./shortcuts";

/**
 * 外壳能力：各平台标题栏 / 菜单的承载方式。
 * 窗口控制不在能力矩阵内——由系统（macOS 红绿灯）或 tauri-plugin-decoration
 * 内嵌控制按钮（Windows/Linux，含 Win11 Snap Layout 热区）承载，标题栏
 * 布局通过插件的 clearance CSS 变量避让，无需应用分支。
 */
export interface ShellCapabilities {
  /** 应用菜单由系统菜单栏承载（macOS 桌面端）；false 时标题栏内自绘 web 菜单。 */
  nativeMenu: boolean;
  /** 命令面板修饰键提示（⌘K / Ctrl+K）。 */
  paletteShortcutLabel: string;
}

/**
 * 按平台与运行时解析能力矩阵：
 * - macOS + Tauri：系统菜单栏 + Overlay 标题栏 + 红绿灯；
 * - Windows/Linux + Tauri：无边框窗口，标题栏自绘菜单与窗口控制；
 * - 浏览器预览（任意平台）：菜单仍需自绘（系统菜单栏属于浏览器），
 *   无窗口控制，macOS 下无 Overlay（不存在系统标题栏可延伸）。
 */
export function resolveShellCapabilities(platform: Platform, tauri: boolean): ShellCapabilities {
  const paletteShortcutLabel = shortcutLabel(SHORTCUTS.commandPalette, platform);
  if (platform === "macos") {
    return {
      nativeMenu: tauri,
      paletteShortcutLabel,
    };
  }
  return {
    nativeMenu: false,
    paletteShortcutLabel,
  };
}
