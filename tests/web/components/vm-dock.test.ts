import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import VmDock from "../../../src-web/components/vm-dock/VmDock.vue";
import { useVmStore } from "../../../src-web/stores/vm";

describe("VmDock", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("默认隐藏（v-show），store 置真后显示", async () => {
    const wrapper = mount(VmDock, {
      global: { plugins: [pinia] },
      slots: { default: "<p>VM 面板</p>" },
    });
    const vm = useVmStore();
    const dock = wrapper.find("div");
    expect(dock.element.style.display).toBe("none");
    expect(dock.classes()).toContain("absolute");

    vm.vmPanelVisible = true;
    await wrapper.vm.$nextTick();
    expect(dock.element.style.display).not.toBe("none");
    expect(dock.text()).toContain("VM 面板");
  });

  it("初始即可见的场景（菜单已展开后挂载）", () => {
    useVmStore().vmPanelVisible = true;
    const wrapper = mount(VmDock, { global: { plugins: [pinia] } });
    expect(wrapper.find("div").element.style.display).not.toBe("none");
  });
});
