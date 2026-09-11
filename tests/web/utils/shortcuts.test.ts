import { describe, expect, it, vi } from "vitest";
import { matchesShortcut, shortcutLabel, SHORTCUTS } from "../../../src-web/utils/shortcuts";

/** 构造键盘事件形状（只含匹配所需字段）。 */
const key = (
  value: string,
  modifiers: { metaKey?: boolean; ctrlKey?: boolean; shiftKey?: boolean } = {},
) => ({
  key: value,
  metaKey: modifiers.metaKey ?? false,
  ctrlKey: modifiers.ctrlKey ?? false,
  shiftKey: modifiers.shiftKey ?? false,
});

describe("SHORTCUTS 定义表", () => {
  it("面板与文件类快捷键齐全", () => {
    expect(SHORTCUTS.commandPalette).toEqual({ key: "k" });
    expect(SHORTCUTS.fileNew).toEqual({ key: "n" });
    expect(SHORTCUTS.fileOpen).toEqual({ key: "o" });
    expect(SHORTCUTS.fileSave).toEqual({ key: "s" });
    expect(SHORTCUTS.fileSaveAs).toEqual({ key: "s", shift: true });
  });
});

describe("matchesShortcut", () => {
  it("macOS 只认 ⌘，Ctrl 不触发", () => {
    expect(matchesShortcut(key("k", { metaKey: true }), SHORTCUTS.commandPalette, "macos")).toBe(
      true,
    );
    expect(matchesShortcut(key("k", { ctrlKey: true }), SHORTCUTS.commandPalette, "macos")).toBe(
      false,
    );
  });

  it("Windows / Linux 只认 Ctrl，⌘ 不触发", () => {
    expect(matchesShortcut(key("k", { ctrlKey: true }), SHORTCUTS.commandPalette, "windows")).toBe(
      true,
    );
    expect(matchesShortcut(key("k", { metaKey: true }), SHORTCUTS.commandPalette, "linux")).toBe(
      false,
    );
  });

  it("键名不同一律不触发", () => {
    expect(matchesShortcut(key("s", { metaKey: true }), SHORTCUTS.commandPalette, "macos")).toBe(
      false,
    );
  });

  it("Shift 定义严格匹配（大小写键名不敏感）", () => {
    expect(
      matchesShortcut(key("S", { metaKey: true, shiftKey: true }), SHORTCUTS.fileSaveAs, "macos"),
    ).toBe(true);
    expect(matchesShortcut(key("s", { metaKey: true }), SHORTCUTS.fileSaveAs, "macos")).toBe(false);
    expect(
      matchesShortcut(key("S", { ctrlKey: true, shiftKey: true }), SHORTCUTS.fileSaveAs, "windows"),
    ).toBe(true);
  });

  it("未传平台时按运行环境解析（UA → macos）", () => {
    vi.stubGlobal("navigator", { userAgent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)" });
    try {
      expect(matchesShortcut(key("k", { metaKey: true }), SHORTCUTS.commandPalette)).toBe(true);
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it("navigator 不可用时按 macOS 兜底", () => {
    vi.stubGlobal("navigator", undefined);
    try {
      expect(matchesShortcut(key("k", { metaKey: true }), SHORTCUTS.commandPalette)).toBe(true);
    } finally {
      vi.unstubAllGlobals();
    }
  });
});

describe("shortcutLabel", () => {
  it("macOS 用 ⌘/⇧⌘ 符号", () => {
    expect(shortcutLabel(SHORTCUTS.commandPalette, "macos")).toBe("⌘K");
    expect(shortcutLabel(SHORTCUTS.fileSave, "macos")).toBe("⌘S");
    expect(shortcutLabel(SHORTCUTS.fileSaveAs, "macos")).toBe("⇧⌘S");
  });

  it("Windows / Linux 用 Ctrl 写法", () => {
    expect(shortcutLabel(SHORTCUTS.commandPalette, "windows")).toBe("Ctrl+K");
    expect(shortcutLabel(SHORTCUTS.fileSaveAs, "windows")).toBe("Ctrl+Shift+S");
    expect(shortcutLabel(SHORTCUTS.fileSaveAs, "linux")).toBe("Ctrl+Shift+S");
  });
});
