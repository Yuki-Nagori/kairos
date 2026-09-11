import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { resolveShellCapabilities } from "../../../src-web/utils/shell";
import { useShell } from "../../../src-web/composables/useShell";

describe("resolveShellCapabilities 能力矩阵", () => {
  it("macOS + Tauri：菜单由系统菜单栏承载，行首避开红绿灯", () => {
    const shell = resolveShellCapabilities("macos", true);
    expect(shell).toEqual({
      nativeMenu: true,
      paletteShortcutLabel: "⌘K",
      titleBarPaddingLeft: "80px",
      titleBarPaddingRight: "12px",
    });
  });

  it("Windows/Linux：无边框窗口，标题栏内自绘菜单并按插件 clearance 避让", () => {
    const shell = resolveShellCapabilities("windows", true);
    expect(shell).toEqual({
      nativeMenu: false,
      paletteShortcutLabel: "Ctrl+K",
      titleBarPaddingLeft: "max(12px, var(--tauri-plugin-decoration-left-clearance, 0px))",
      titleBarPaddingRight: "max(12px, var(--tauri-plugin-decoration-right-clearance, 0px))",
    });
    expect(resolveShellCapabilities("linux", true).nativeMenu).toBe(false);
  });

  it("浏览器预览：系统菜单栏属于浏览器，任意平台均自绘 web 菜单且无插件避让", () => {
    for (const platform of ["macos", "windows", "linux"] as const) {
      const shell = resolveShellCapabilities(platform, false);
      expect(shell.nativeMenu).toBe(false);
      expect(shell.titleBarPaddingLeft).toBe("12px");
      expect(shell.titleBarPaddingRight).toBe("12px");
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
        paletteShortcutLabel: "Ctrl+K",
        titleBarPaddingLeft: "12px",
        titleBarPaddingRight: "12px",
      });
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
