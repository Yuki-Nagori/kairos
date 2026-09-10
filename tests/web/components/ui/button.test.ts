import { describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import UiButton from "../../../../src-web/components/ui/Button.vue";

describe("UiButton", () => {
  it("默认 ghost 描边样式并渲染插槽文案", () => {
    const wrapper = mount(UiButton, { slots: { default: "重新探测" } });
    const button = wrapper.find("button");
    expect(button.text()).toBe("重新探测");
    expect(button.attributes("type")).toBe("button");
    expect(button.classes()).toContain("border");
    expect(button.classes()).not.toContain("bg-emerald-500");
  });

  it("primary / danger 变体各用各的配色", () => {
    const primary = mount(UiButton, { props: { variant: "primary" } });
    expect(primary.find("button").classes()).toContain("bg-emerald-500");

    const danger = mount(UiButton, { props: { variant: "danger" } });
    expect(danger.find("button").classes()).toContain("hover:text-red-400");
  });

  it("disabled 与 click 经原生透传生效", async () => {
    const onClick = vi.fn();
    const disabled = mount(UiButton, { attrs: { onClick, disabled: true } });
    expect(disabled.find("button").attributes("disabled")).toBeDefined();
    await disabled.find("button").trigger("click");
    expect(onClick).not.toHaveBeenCalled();

    const enabled = mount(UiButton, { attrs: { onClick } });
    await enabled.find("button").trigger("click");
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
