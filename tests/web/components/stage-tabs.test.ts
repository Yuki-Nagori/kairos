import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import StageTabs from "../../../src-web/components/StageTabs.vue";
import { appStore, initialAppState } from "../../../src-web/state";

const LABELS = ["主页", "几何", "网格", "工艺", "求解", "结果", "报告"];

describe("StageTabs", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
  });

  it("渲染七个阶段选项卡，当前阶段高亮", () => {
    const wrapper = mount(StageTabs);
    const tabs = wrapper.findAll("button");
    expect(tabs.map((tab) => tab.text())).toEqual(LABELS);
    expect(tabs[0]?.classes()).toContain("text-emerald-400");
    expect(tabs[1]?.classes()).not.toContain("text-emerald-400");
  });

  it("点击切换 store.stage", async () => {
    const wrapper = mount(StageTabs);
    await wrapper.findAll("button")[3]!.trigger("click");
    expect(appStore.get().stage).toBe("process");
  });

  it("store 变化驱动高亮，卸载后停止订阅", async () => {
    const wrapper = mount(StageTabs);
    appStore.set({ stage: "solve" });
    await wrapper.vm.$nextTick();
    expect(wrapper.findAll("button")[4]!.classes()).toContain("text-emerald-400");

    wrapper.unmount();
    appStore.set({ stage: "home" }); // 不应抛错（订阅已清理）
  });
});
