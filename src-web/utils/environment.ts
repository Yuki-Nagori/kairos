/** 当前是否运行在 Tauri WebView 内（而非普通浏览器）。 */
export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** 桌面平台（快捷键提示与匹配按平台区分）。 */
export type Platform = "macos" | "windows" | "linux";

/** 从 userAgent 解析平台：iPadOS 15+ 的 UA 也报告 Mac，归入 macOS 符合其键位习惯。 */
export function detectPlatform(userAgent: string): Platform {
  if (/Mac/i.test(userAgent)) {
    return "macos";
  }
  if (/Win/i.test(userAgent)) {
    return "windows";
  }
  return "linux";
}

/** 当前平台：标题栏形态（macOS 系统栏 / 其余自绘）与快捷键提示按它分支。 */
export function currentPlatform(): Platform {
  if (typeof navigator === "undefined") {
    return "macos";
  }
  return detectPlatform(navigator.userAgent);
}

/** 事件是否命中当前平台的命令面板快捷键（macOS ⌘K / Windows·Linux Ctrl+K）。 */
export function isCommandPaletteShortcut(
  event: { metaKey: boolean; ctrlKey: boolean; key: string },
  platform: Platform = currentPlatform(),
): boolean {
  if (event.key.toLowerCase() !== "k") {
    return false;
  }
  return platform === "macos" ? event.metaKey : event.ctrlKey;
}

/** 命令面板快捷键的展示文案（搜索框内提示）。 */
export function commandPaletteShortcutLabel(platform: Platform = currentPlatform()): string {
  return platform === "macos" ? "⌘K" : "Ctrl+K";
}

/** 通用加速键展示：macOS 用 ⌘/⇧ 组合符号，其余平台用 Ctrl/Shift 写法。 */
export function acceleratorLabel(
  macLabel: string,
  otherLabel: string,
  platform: Platform = currentPlatform(),
): string {
  return platform === "macos" ? macLabel : otherLabel;
}
