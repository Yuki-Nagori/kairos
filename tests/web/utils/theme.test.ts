import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  THEME_CHANGED_EVENT,
  cycleTheme,
  getActiveTheme,
  initTheme,
  themeRgb,
  themeVar,
} from "../../../src-web/utils/theme";

const STORAGE_KEY = "kairos:theme:active";

function injectedStyles() {
  return Array.from(document.head.querySelectorAll<HTMLElement>("style[data-kairos-theme]"));
}

beforeEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
  injectedStyles().forEach((style) => style.remove());
});
afterEach(() => {
  localStorage.clear();
  injectedStyles().forEach((style) => style.remove());
  document.documentElement.removeAttribute("data-theme");
  vi.unstubAllGlobals();
});

describe("initTheme", () => {
  it("把全部主题 CSS 注入 head 并默认应用首个主题", () => {
    const theme = initTheme();
    expect(theme).toBe("dark");
    expect(getActiveTheme()).toBe("dark");
    const names = injectedStyles().map((style) => style.dataset.kairosTheme);
    expect(names).toEqual(["dark", "light"]);
  });

  it("恢复持久化的有效偏好", () => {
    localStorage.setItem(STORAGE_KEY, '"light"');
    expect(initTheme()).toBe("light");
    expect(getActiveTheme()).toBe("light");
  });

  it("持久化偏好非法时回落首个主题", () => {
    localStorage.setItem(STORAGE_KEY, "banana");
    expect(initTheme()).toBe("dark");
  });
});

describe("getActiveTheme / cycleTheme", () => {
  it("未初始化时回落主题目录首项", () => {
    expect(getActiveTheme()).toBe("dark");
  });

  it("循环切换主题并持久化，末尾回绕", () => {
    initTheme();
    expect(cycleTheme()).toBe("light");
    expect(getActiveTheme()).toBe("light");
    expect(localStorage.getItem(STORAGE_KEY)).toBe('"light"');
    expect(cycleTheme()).toBe("dark");
    expect(localStorage.getItem(STORAGE_KEY)).toBe('"dark"');
  });

  it("每次切换广播 THEME_CHANGED_EVENT", () => {
    initTheme();
    const listener = vi.fn();
    window.addEventListener(THEME_CHANGED_EVENT, listener);
    try {
      cycleTheme();
      expect(listener).toHaveBeenCalledTimes(1);
    } finally {
      window.removeEventListener(THEME_CHANGED_EVENT, listener);
    }
  });

  it("当前主题不在目录中时回绕到首项", () => {
    document.documentElement.dataset.theme = "custom";
    expect(cycleTheme()).toBe("dark");
  });
});

describe("themeVar / themeRgb", () => {
  it("取文档根的 CSS 变量并 trim", () => {
    vi.stubGlobal("getComputedStyle", () => ({
      getPropertyValue: (name: string) => (name === "--c-viewport-bg" ? "  #0d1420  " : ""),
    }));
    expect(themeVar("--c-viewport-bg", "#000000")).toBe("#0d1420");
  });

  it("变量缺失时用回退值；无 getComputedStyle 的环境同样回退", () => {
    vi.stubGlobal("getComputedStyle", () => ({ getPropertyValue: () => "" }));
    expect(themeVar("--c-missing", "#111111")).toBe("#111111");
    vi.stubGlobal("getComputedStyle", undefined);
    expect(themeVar("--c-viewport-bg", "#111111")).toBe("#111111");
  });

  it("颜色变量解析为 0..1 分量；非 #rrggbb 形式（或缺失）回落回退色", () => {
    vi.stubGlobal("getComputedStyle", () => ({
      getPropertyValue: (name: string) => (name === "--c-hot" ? "#ff8000" : "rgb(1, 2, 3)"),
    }));
    expect(themeRgb("--c-hot", [0, 0, 0])).toEqual([1, 128 / 255, 0]);
    // rgb() 形式读不出分量：宁可给出确定的兜底色，也不要 NaN
    expect(themeRgb("--c-other", [0.1, 0.2, 0.3])).toEqual([0.1, 0.2, 0.3]);
  });
});

describe("空主题目录的防御回退", () => {
  it("主题目录为空时全部回退到 dark（独立模块实例）", async () => {
    // THEME_STYLES 在模块加载期由 import.meta.glob 固化，正常装载下非空；
    // 拦截 Object.entries 使 glob 结果映射为空，再重新导入模块驱动空目录分支。
    vi.resetModules();
    const originalEntries = Object.entries;
    const entriesRef = Object as unknown as {
      entries: (target: unknown) => [string, unknown][];
    };
    entriesRef.entries = (target) => {
      if (
        target !== null &&
        typeof target === "object" &&
        originalEntries(target as Record<string, unknown>).some(
          ([key]) => key.includes("/theme/") && key.endsWith(".css"),
        )
      ) {
        return [];
      }
      return originalEntries(target as Record<string, unknown>);
    };
    try {
      const isolated = await import("../../../src-web/utils/theme");
      expect(isolated.getActiveTheme()).toBe("dark"); // THEME_STYLES[0]?.[0] ?? "dark"
      expect(isolated.initTheme()).toBe("dark"); // names[0] ?? "dark"，且不注入任何样式
      expect(injectedStyles()).toHaveLength(0);
      expect(isolated.cycleTheme()).toBe("dark"); // 空目录回绕：?? active
    } finally {
      entriesRef.entries = (target) => originalEntries(target as Record<string, unknown>);
    }
  });
});
