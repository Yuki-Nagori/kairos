import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import StageTabs from "../../../src-web/components/StageTabs.vue";
import { useAppStore } from "../../../src-web/stores/app";

const LABELS = ["主页", "几何", "网格", "工艺", "求解", "结果", "报告"];

describe("StageTabs", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("渲染七个阶段选项卡，当前阶段高亮", () => {
    const wrapper = mount(StageTabs, { global: { plugins: [pinia] } });
    const tabs = wrapper.findAll("button");
    expect(tabs.map((tab) => tab.text())).toEqual(LABELS);
    expect(tabs[0]?.classes()).toContain("text-emerald-400");
    expect(tabs[1]?.classes()).not.toContain("text-emerald-400");
  });

  it("点击切换 store.stage", async () => {
    const wrapper = mount(StageTabs, { global: { plugins: [pinia] } });
    await wrapper.findAll("button")[3]!.trigger("click");
    expect(useAppStore().stage).toBe("process");
  });

  it("store 变化驱动高亮，卸载后停止订阅", async () => {
    const app = useAppStore();
    const wrapper = mount(StageTabs, { global: { plugins: [pinia] } });
    app.stage = "solve";
    await wrapper.vm.$nextTick();
    expect(wrapper.findAll("button")[4]!.classes()).toContain("text-emerald-400");

    wrapper.unmount();
    app.stage = "home"; // 不应抛错（订阅已清理）
  });
});
