import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import StatusBar from "../../../src-web/components/StatusBar.vue";
import AppHeader from "../../../src-web/components/AppHeader.vue";
import { appStore, initialAppState } from "../../../src-web/state";
import { getActiveTheme } from "../../../src-web/theme";
import type { SystemInfo } from "../../../src-web/types";

const info: SystemInfo = { name: "kairos", version: "0.1.0", os: "macos" };

describe("StatusBar 状态三态优先级", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
  });

  it("空闲时显示版本与平台", () => {
    appStore.set({ info });
    const wrapper = mount(StatusBar);
    expect(wrapper.find("span").text()).toContain("v0.1.0");
  });

  it("忙碌优先于版本信息", () => {
    appStore.set({ info, busy: "正在求解…" });
    const wrapper = mount(StatusBar);
    expect(wrapper.find("span").text()).toBe("正在求解…");
  });

  it("错误优先于忙碌与版本", () => {
    appStore.set({
      info,
      busy: "正在求解…",
      error: { message: "迭代不收敛", info: false },
    });
    const wrapper = mount(StatusBar);
    expect(wrapper.find("span").text()).toBe("迭代不收敛");
  });
});

describe("StatusBar Shell 入口", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
  });

  it("点击切换虚拟机面板显隐并更新文案", async () => {
    const wrapper = mount(StatusBar);
    const button = wrapper.find("button");
    // 文字提示在左，shell 图标（内联 SVG）在右
    expect(button.text()).toContain("Shell 环境");
    expect(button.find("svg").exists()).toBe(true);
    expect(appStore.get().vmPanelVisible).toBe(false);

    await button.trigger("click");
    expect(appStore.get().vmPanelVisible).toBe(true);
    expect(button.text()).toContain("点击收起");

    await button.trigger("click");
    expect(appStore.get().vmPanelVisible).toBe(false);
  });
});

describe("AppHeader", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
  });

  it("品牌信息与七个阶段选项卡", () => {
    const wrapper = mount(AppHeader);
    expect(wrapper.find("h1").text()).toBe("Kairos");
    const tabCount = wrapper.findAll("button").length - 1; // 末位是主题按钮
    expect(tabCount).toBe(7);
  });

  it("主题按钮随全局主题切换图标提示", async () => {
    const wrapper = mount(AppHeader);
    const themeButton = wrapper.findAll("button").at(-1)!;
    const before = getActiveTheme();
    expect(themeButton.attributes("title")).toBe(
      before === "light" ? "切换到浅色主题" : "切换到深色主题",
    );

    await themeButton.trigger("click");
    expect(getActiveTheme()).not.toBe(before);
    expect(themeButton.attributes("title")).toBe(
      getActiveTheme() === "light" ? "切换到浅色主题" : "切换到深色主题",
    );
  });
});
