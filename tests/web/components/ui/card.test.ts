import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import Card from "../../../../src-web/components/ui/UiCard.vue";

describe("Card", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("渲染标题 / 图标插槽 / 状态行 / 默认插槽", () => {
    const wrapper = mount(Card, {
      props: { title: "依赖状态", statusHint: "全部就绪" },
      slots: { icon: "<b data-test='icon'>◆</b>", default: "<p>正文内容</p>" },
    });
    expect(wrapper.find("h2").text()).toBe("依赖状态");
    expect(wrapper.find("[data-test='icon']").exists()).toBe(true);
    expect(wrapper.text()).toContain("全部就绪");
    expect(wrapper.text()).toContain("正文内容");
  });

  it("提供 refreshLabel 时渲染刷新按钮并上抛 refresh", async () => {
    const wrapper = mount(Card, {
      props: { title: "虚拟机", refreshLabel: "重新探测" },
    });
    await wrapper.find("button").trigger("click");
    expect(wrapper.emitted("refresh")).toHaveLength(1);
  });

  it("折叠切换写 localStorage，重新挂载后保持", async () => {
    const wrapper = mount(Card, { props: { title: "流程引导", collapsible: true } });
    expect(wrapper.text()).toContain("▾");
    await wrapper.find("h2").trigger("click");
    expect(localStorage.getItem("kairos-panel:流程引导")).toBe("1");
    expect(wrapper.text()).toContain("▸");

    const remounted = mount(Card, { props: { title: "流程引导", collapsible: true } });
    expect(remounted.text()).toContain("▸");
  });

  it("不可折叠卡片没有箭头，点击标题不写存储", async () => {
    const wrapper = mount(Card, { props: { title: "静态卡片" } });
    expect(wrapper.text()).not.toContain("▾");
    await wrapper.find("h2").trigger("click");
    expect(localStorage.getItem("kairos-panel:静态卡片")).toBeNull();
  });
});
