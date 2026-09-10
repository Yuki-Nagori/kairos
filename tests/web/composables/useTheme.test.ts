import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { defineComponent, h } from "vue";
import {
  THEME_CHANGED_EVENT,
  cycleTheme,
  getActiveTheme,
  initTheme,
  useTheme,
} from "../../../src-web/composables/useTheme";

const STORAGE_KEY = "kairos-theme";

function injectedStyles() {
  return Array.from(document.head.querySelectorAll<HTMLElement>("style[data-kairos-theme]"));
}

/** useTheme 依赖组件实例（onUnmounted），用最小宿主组件挂载。 */
function mountThemeHost() {
  let themeRef!: ReturnType<typeof useTheme>["theme"];
  const wrapper = mount(
    defineComponent({
      setup() {
        const { theme, cycleTheme: cycle } = useTheme();
        themeRef = theme;
        return () => h("button", { onClick: () => cycle() }, theme.value);
      },
    }),
  );
  return { wrapper, themeRef };
}

describe("initTheme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    injectedStyles().forEach((style) => style.remove());
  });
  afterEach(() => {
    localStorage.clear();
    injectedStyles().forEach((style) => style.remove());
    document.documentElement.removeAttribute("data-theme");
  });

  it("把全部主题 CSS 注入 head 并默认应用首个主题", () => {
    const theme = initTheme();
    expect(theme).toBe("dark");
    expect(getActiveTheme()).toBe("dark");
    const names = injectedStyles().map((style) => style.dataset.kairosTheme);
    expect(names).toEqual(["dark", "light"]);
  });

  it("恢复持久化的有效偏好", () => {
    localStorage.setItem(STORAGE_KEY, "light");
    expect(initTheme()).toBe("light");
    expect(getActiveTheme()).toBe("light");
  });

  it("持久化偏好非法时回落首个主题", () => {
    localStorage.setItem(STORAGE_KEY, "banana");
    expect(initTheme()).toBe("dark");
  });
});

describe("getActiveTheme / cycleTheme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    injectedStyles().forEach((style) => style.remove());
  });
  afterEach(() => {
    localStorage.clear();
    injectedStyles().forEach((style) => style.remove());
    document.documentElement.removeAttribute("data-theme");
  });

  it("未初始化时回落主题目录首项", () => {
    expect(getActiveTheme()).toBe("dark");
  });

  it("循环切换主题并持久化，末尾回绕", () => {
    initTheme();
    expect(cycleTheme()).toBe("light");
    expect(getActiveTheme()).toBe("light");
    expect(localStorage.getItem(STORAGE_KEY)).toBe("light");
    expect(cycleTheme()).toBe("dark");
    expect(localStorage.getItem(STORAGE_KEY)).toBe("dark");
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

describe("useTheme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    injectedStyles().forEach((style) => style.remove());
    initTheme();
  });
  afterEach(() => {
    localStorage.clear();
    injectedStyles().forEach((style) => style.remove());
    document.documentElement.removeAttribute("data-theme");
  });

  it("初始值为当前主题，切换事件同步更新", async () => {
    const { wrapper } = mountThemeHost();
    expect(wrapper.text()).toBe("dark");
    await wrapper.find("button").trigger("click");
    // cycleTheme 广播事件 → 监听器同步 ref。
    expect(wrapper.text()).toBe("light");
    expect(getActiveTheme()).toBe("light");
  });

  it("卸载后清理事件监听，ref 不再同步", () => {
    const { wrapper, themeRef } = mountThemeHost();
    expect(themeRef.value).toBe("dark");
    wrapper.unmount();
    cycleTheme();
    // 全局主题已切换，但监听已清理，卸载组件的 ref 不再跟随。
    expect(getActiveTheme()).toBe("light");
    expect(themeRef.value).toBe("dark");
  });
});

describe("空主题目录的防御回退", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    injectedStyles().forEach((style) => style.remove());
  });
  afterEach(() => {
    localStorage.clear();
    injectedStyles().forEach((style) => style.remove());
    document.documentElement.removeAttribute("data-theme");
  });

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
      const isolated = await import("../../../src-web/composables/useTheme");
      expect(isolated.getActiveTheme()).toBe("dark"); // THEME_STYLES[0]?.[0] ?? "dark"
      expect(isolated.initTheme()).toBe("dark"); // names[0] ?? "dark"，且不注入任何样式
      expect(injectedStyles()).toHaveLength(0);
      expect(isolated.cycleTheme()).toBe("dark"); // 空目录回绕：?? active
    } finally {
      entriesRef.entries = (target) => originalEntries(target as Record<string, unknown>);
    }
  });
});
