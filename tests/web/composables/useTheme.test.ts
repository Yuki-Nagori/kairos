import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { defineComponent, h } from "vue";
import { useTheme } from "../../../src-web/composables/useTheme";
import { cycleTheme, getActiveTheme, initTheme } from "../../../src-web/utils/theme";

const STORAGE_KEY = "kairos:theme:active";

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

  it("持久化偏好决定初始值", () => {
    localStorage.setItem(STORAGE_KEY, '"light"');
    initTheme();
    const { wrapper } = mountThemeHost();
    expect(wrapper.text()).toBe("light");
  });
});
