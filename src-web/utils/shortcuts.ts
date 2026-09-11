/**
 * 快捷键注册表：定义、匹配、展示文案的单一来源（平台差异在此收敛）。
 * 修饰键平台约定：macOS ⌘（meta），Windows/Linux Ctrl；Shift 可叠加。
 * 消费方式：匹配用 matchesShortcut(event, SHORTCUTS.x)，展示用
 * shortcutLabel(SHORTCUTS.x)；macOS 原生菜单加速器与此表一一对应
 * （Rust 侧 CmdOrCtrl 写法，改键位需两处同步）。
 */
import { currentPlatform, type Platform } from "./environment";

/** 快捷键定义：key 为小写键名，shift 表示需叠加 Shift。 */
interface Shortcut {
  key: string;
  shift?: boolean;
}

/** 应用快捷键定义表：id 与菜单动作 id 对应。 */
export const SHORTCUTS = {
  commandPalette: { key: "k" },
  fileNew: { key: "n" },
  fileOpen: { key: "o" },
  fileSave: { key: "s" },
  fileSaveAs: { key: "s", shift: true },
} as const satisfies Record<string, Shortcut>;

/** 键盘事件是否命中当前平台的该快捷键（修饰键与 Shift 均严格匹配）。 */
export function matchesShortcut(
  event: { key: string; metaKey: boolean; ctrlKey: boolean; shiftKey: boolean },
  shortcut: Shortcut,
  platform: Platform = currentPlatform(),
): boolean {
  if (event.key.toLowerCase() !== shortcut.key) {
    return false;
  }
  const main = platform === "macos" ? event.metaKey : event.ctrlKey;
  if (!main) {
    return false;
  }
  return (shortcut.shift ?? false) === event.shiftKey;
}

/** 快捷键展示文案：macOS ⌘S / ⇧⌘S，其余 Ctrl+S / Ctrl+Shift+S。 */
export function shortcutLabel(shortcut: Shortcut, platform: Platform = currentPlatform()): string {
  const key = shortcut.key.toUpperCase();
  if (platform === "macos") {
    return shortcut.shift ? `⇧⌘${key}` : `⌘${key}`;
  }
  return shortcut.shift ? `Ctrl+Shift+${key}` : `Ctrl+${key}`;
}
