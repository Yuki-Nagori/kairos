import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import StatusBar from "../../../src-web/components/status-bar/StatusBar.vue";
import { useAppStore } from "../../../src-web/stores/app";
import { useVmStore } from "../../../src-web/stores/vm";
import type { SystemInfo } from "../../../src-web/types";

const info: SystemInfo = { name: "kairos", version: "0.1.0", os: "macos" };

describe("StatusBar 状态三态优先级", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("空闲时显示版本与平台", () => {
    useAppStore().info = info;
    const wrapper = mount(StatusBar, { global: { plugins: [pinia] } });
    expect(wrapper.find("span").text()).toContain("v0.1.0");
  });

  it("忙碌优先于版本信息", () => {
    const app = useAppStore();
    app.info = info;
    app.busy = "正在求解…";
    const wrapper = mount(StatusBar, { global: { plugins: [pinia] } });
    expect(wrapper.find("span").text()).toBe("正在求解…");
  });

  it("错误优先于忙碌与版本", () => {
    const app = useAppStore();
    app.info = info;
    app.busy = "正在求解…";
    app.error = { message: "迭代不收敛", info: false };
    const wrapper = mount(StatusBar, { global: { plugins: [pinia] } });
    expect(wrapper.find("span").text()).toBe("迭代不收敛");
  });

  it("IPC 提示型错误用中性色而非错误红", () => {
    const app = useAppStore();
    app.info = info;
    app.error = { message: "未连接 Tauri 运行时", info: true };
    const wrapper = mount(StatusBar, { global: { plugins: [pinia] } });
    expect(wrapper.find("span").text()).toBe("未连接 Tauri 运行时");
    expect(wrapper.find("span").classes()).toContain("text-zinc-400");
    expect(wrapper.find("span").classes()).not.toContain("text-red-400");
  });
});

describe("StatusBar Shell 入口", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("点击切换虚拟机面板显隐并更新文案", async () => {
    const wrapper = mount(StatusBar, { global: { plugins: [pinia] } });
    const vm = useVmStore();
    const button = wrapper.find("button");
    // 文字提示在左，shell 图标（内联 SVG）在右
    expect(button.text()).toContain("Shell 环境");
    expect(button.find("svg").exists()).toBe(true);
    expect(vm.vmPanelVisible).toBe(false);

    await button.trigger("click");
    expect(vm.vmPanelVisible).toBe(true);
    expect(button.text()).toContain("点击收起");

    await button.trigger("click");
    expect(vm.vmPanelVisible).toBe(false);
  });
});
