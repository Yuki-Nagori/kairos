import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import VmDock from "../../../src-web/components/VmDock.vue";
import { appStore, initialAppState } from "../../../src-web/state";

describe("VmDock", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
  });

  it("默认隐藏（v-show），store 置真后显示", async () => {
    const wrapper = mount(VmDock, { slots: { default: "<p>VM 面板</p>" } });
    const dock = wrapper.find("div");
    expect(dock.element.style.display).toBe("none");
    expect(dock.classes()).toContain("absolute");

    appStore.set({ vmPanelVisible: true });
    await wrapper.vm.$nextTick();
    expect(dock.element.style.display).not.toBe("none");
    expect(dock.text()).toContain("VM 面板");
  });

  it("初始即可见的场景（菜单已展开后挂载）", () => {
    appStore.set({ vmPanelVisible: true });
    const wrapper = mount(VmDock);
    expect(wrapper.find("div").element.style.display).not.toBe("none");
  });
});
