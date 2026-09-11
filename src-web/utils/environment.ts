/** 平台运行环境：运行时判定与平台判定（快捷键定义/匹配/文案见 utils/shortcuts）。 */

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

/** 当前是否运行在 Tauri WebView 内（而非普通浏览器）。 */
export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
