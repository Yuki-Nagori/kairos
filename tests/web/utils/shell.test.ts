import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { resolveShellCapabilities } from "../../../src-web/utils/shell";
import { useShell } from "../../../src-web/composables/useShell";

describe("resolveShellCapabilities 能力矩阵", () => {
  it("macOS + Tauri：系统菜单栏 / 红绿灯 / Overlay，无自绘窗口控制", () => {
    const shell = resolveShellCapabilities("macos", true);
    expect(shell).toEqual({
      nativeMenu: true,
      nativeWindowControls: true,
      overlayTitleBar: true,
      windowControls: false,
      paletteShortcutLabel: "⌘K",
    });
  });

  it("Windows/Linux + Tauri：无边框窗口自绘菜单与窗口控制", () => {
    const shell = resolveShellCapabilities("windows", true);
    expect(shell).toEqual({
      nativeMenu: false,
      nativeWindowControls: false,
      overlayTitleBar: false,
      windowControls: true,
      paletteShortcutLabel: "Ctrl+K",
    });
    expect(resolveShellCapabilities("linux", true).windowControls).toBe(true);
  });

  it("浏览器预览：无系统菜单与窗口控制，macOS 亦无 Overlay 可延伸", () => {
    for (const platform of ["macos", "windows", "linux"] as const) {
      const shell = resolveShellCapabilities(platform, false);
      expect(shell.nativeMenu).toBe(false);
      expect(shell.nativeWindowControls).toBe(false);
      expect(shell.overlayTitleBar).toBe(false);
      expect(shell.windowControls).toBe(false);
    }
  });

  it("快捷键提示随平台（macOS ⌘K / 其余 Ctrl+K）", () => {
    expect(resolveShellCapabilities("macos", true).paletteShortcutLabel).toBe("⌘K");
    expect(resolveShellCapabilities("windows", false).paletteShortcutLabel).toBe("Ctrl+K");
  });
});

describe("useShell", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("按运行时 UA 与 Tauri 环境解析能力（Windows UA 无 Tauri）", () => {
    vi.stubGlobal("navigator", { userAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64)" });
    try {
      expect(useShell()).toEqual({
        nativeMenu: false,
        nativeWindowControls: false,
        overlayTitleBar: false,
        windowControls: false,
        paletteShortcutLabel: "Ctrl+K",
      });
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
