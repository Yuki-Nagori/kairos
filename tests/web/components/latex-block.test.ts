import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import LatexBlock from "../../../src-web/components/LatexBlock.vue";

describe("LatexBlock", () => {
  it("行间模式渲染 katex 公式并带展示态样式", () => {
    const wrapper = mount(LatexBlock, { props: { tex: "E = mc^2" } });
    expect(wrapper.find(".katex-display").exists()).toBe(true);
    expect(wrapper.find(".katex").exists()).toBe(true);
    expect(wrapper.find("div").classes()).toContain("text-center");
  });

  it("行内模式不换行展示", () => {
    const wrapper = mount(LatexBlock, {
      props: { tex: "\\eta_0", displayMode: false },
    });
    expect(wrapper.find(".katex-display").exists()).toBe(false);
    expect(wrapper.find(".katex").exists()).toBe(true);
    expect(wrapper.find("div").classes()).toHaveLength(0);
  });

  it("非法公式不抛异常（throwOnError 关闭，原样回显输入）", () => {
    const wrapper = mount(LatexBlock, { props: { tex: "\\notacommand{x}" } });
    expect(wrapper.find(".katex").exists()).toBe(true);
    expect(wrapper.text()).toContain("\\notacommand");
  });
});
