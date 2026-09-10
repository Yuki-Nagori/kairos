import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import Dropdown from "../../../../src-web/components/ui/UiDropdown.vue";

describe("Dropdown", () => {
  it("渲染插槽选项并回显当前值", () => {
    const wrapper = mount(Dropdown, {
      props: { modelValue: "voxel" },
      slots: {
        default: '<option value="voxel">Voxel</option><option value="gmsh">Gmsh</option>',
      },
    });
    const select = wrapper.find("select");
    expect(select.findAll("option")).toHaveLength(2);
    expect((select.element as HTMLSelectElement).value).toBe("voxel");
  });

  it("切换选项经 update:modelValue 回传", async () => {
    const wrapper = mount(Dropdown, {
      props: { modelValue: "voxel" },
      slots: { default: '<option value="gmsh">Gmsh</option>' },
    });
    await wrapper.find("select").setValue("gmsh");
    expect(wrapper.emitted("update:modelValue")?.at(-1)).toEqual(["gmsh"]);
  });
});
