import { describe, expect, it, vi } from "vitest";
import {
  acceleratorLabel,
  commandPaletteShortcutLabel,
  detectPlatform,
  isCommandPaletteShortcut,
  isTauriRuntime,
} from "../../../src-web/utils/environment";

describe("isTauriRuntime", () => {
  it("无 __TAURI_INTERNALS__ 时为 false", () => {
    expect(isTauriRuntime()).toBe(false);
  });

  it("存在 __TAURI_INTERNALS__ 时为 true", () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    expect(isTauriRuntime()).toBe(true);
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });
});

describe("detectPlatform", () => {
  it("Macintosh / Mac OS X UA 归为 macos", () => {
    expect(detectPlatform("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")).toBe("macos");
  });

  it("Windows UA 归为 windows", () => {
    expect(detectPlatform("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")).toBe("windows");
  });

  it("其余 UA 归为 linux", () => {
    expect(detectPlatform("Mozilla/5.0 (X11; Linux x86_64)")).toBe("linux");
  });
});

describe("isCommandPaletteShortcut", () => {
  const key = (modifiers: { metaKey?: boolean; ctrlKey?: boolean }, value = "k") => ({
    key: value,
    metaKey: modifiers.metaKey ?? false,
    ctrlKey: modifiers.ctrlKey ?? false,
  });

  it("navigator 不可用时按 macOS 兜底", () => {
    vi.stubGlobal("navigator", undefined);
    expect(isCommandPaletteShortcut(key({ metaKey: true }))).toBe(true);
    vi.unstubAllGlobals();
  });

  it("macOS 只认 ⌘（meta），Ctrl+K 不触发", () => {
    expect(isCommandPaletteShortcut(key({ metaKey: true }), "macos")).toBe(true);
    expect(isCommandPaletteShortcut(key({ ctrlKey: true }), "macos")).toBe(false);
    expect(isCommandPaletteShortcut(key({}), "macos")).toBe(false);
  });

  it("Windows / Linux 只认 Ctrl，⌘ 不触发", () => {
    expect(isCommandPaletteShortcut(key({ ctrlKey: true }), "windows")).toBe(true);
    expect(isCommandPaletteShortcut(key({ metaKey: true }), "windows")).toBe(false);
    expect(isCommandPaletteShortcut(key({ ctrlKey: true }), "linux")).toBe(true);
    expect(isCommandPaletteShortcut(key({ metaKey: true }), "linux")).toBe(false);
  });

  it("其他按键一律不触发", () => {
    expect(isCommandPaletteShortcut(key({ metaKey: true }, "s"), "macos")).toBe(false);
    expect(isCommandPaletteShortcut(key({ ctrlKey: true }, "s"), "windows")).toBe(false);
  });
});

describe("快捷键提示文案", () => {
  it("命令面板：macOS 显示 ⌘K，其余显示 Ctrl+K", () => {
    expect(commandPaletteShortcutLabel("macos")).toBe("⌘K");
    expect(commandPaletteShortcutLabel("windows")).toBe("Ctrl+K");
    expect(commandPaletteShortcutLabel("linux")).toBe("Ctrl+K");
  });

  it("通用加速键：macOS 用组合符号，其余用 Ctrl 写法", () => {
    expect(acceleratorLabel("⌘N", "Ctrl+N", "macos")).toBe("⌘N");
    expect(acceleratorLabel("⇧⌘S", "Ctrl+Shift+S", "windows")).toBe("Ctrl+Shift+S");
  });
});
