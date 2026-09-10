import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import TextInput from "../../../../src-web/components/ui/UiTextInput.vue";

describe("TextInput", () => {
  it("渲染 value / placeholder / type 并透传 class", () => {
    const wrapper = mount(TextInput, {
      props: { modelValue: "12", type: "number", placeholder: "核心数" },
      attrs: { class: "w-20", min: "1" },
    });
    const input = wrapper.find("input");
    expect(input.attributes("type")).toBe("number");
    expect(input.attributes("placeholder")).toBe("核心数");
    expect((input.element as HTMLInputElement).value).toBe("12");
    expect(input.classes()).toContain("w-20");
    expect(input.attributes("min")).toBe("1");
  });

  it("输入经 update:modelValue 回传字符串", async () => {
    const wrapper = mount(TextInput, { props: { modelValue: "" } });
    await wrapper.find("input").setValue("48");
    expect(wrapper.emitted("update:modelValue")?.at(-1)).toEqual(["48"]);
  });
});
