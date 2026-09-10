import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import ShellIcon from "../../../../src-web/components/ui/ShellIcon.vue";

describe("ShellIcon", () => {
  it("内联 SVG 且描边继承 currentColor（主题跟随）", () => {
    const wrapper = mount(ShellIcon, { attrs: { class: "h-4 w-4 text-emerald-400" } });
    const svg = wrapper.find("svg");
    expect(svg.exists()).toBe(true);
    expect(svg.attributes("stroke")).toBe("currentColor");
    expect(svg.attributes("aria-hidden")).toBe("true");
    expect(svg.classes()).toContain("h-4");
  });
});
