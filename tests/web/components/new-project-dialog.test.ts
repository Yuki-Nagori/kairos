import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { enableAutoUnmount } from "@vue/test-utils";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import NewProjectDialog from "../../../src-web/components/menu-bar/NewProjectDialog.vue";
import { useNewProjectDialog } from "../../../src-web/components/menu-bar/useNewProjectDialog";
import { useProjectStore } from "../../../src-web/stores/project";
import { useAppStore } from "../../../src-web/stores/app";

vi.mock("../../../src-web/api/project", () => ({
  createProject: vi.fn(),
  defaultProjectPath: vi.fn(
    async (name: string, file: string) => `/home/u/Documents/kairos/${name}/${file}.kairos`,
  ),
  workspaceRootOf: vi.fn(async (path: string) => path.split("/").slice(0, -1).join("/")),
  archiveWorkspaceGeometry: vi.fn(),
  loadWorkspaceGeometry: vi.fn(),
  saveStudyMesh: vi.fn(),
  restoreStudyMesh: vi.fn(),
  listRecentProjects: vi.fn(async () => []),
  loadProjectFile: vi.fn(),
  saveProjectFile: vi.fn(async () => undefined),
}));
vi.mock("../../../src-web/api/system", () => ({ getSystemInfo: vi.fn() }));
vi.mock("../../../src-web/api/materials", () => ({
  listBuiltinMaterials: vi.fn(async () => []),
  listCustomMaterials: vi.fn(async () => []),
}));

function makeProject(name: string) {
  return {
    schemaVersion: 5,
    id: "p-1",
    name,
    createdMs: 1,
    updatedMs: 1,
    studies: [],
    geometries: [],
  };
}

/** 输入框顺序：0 = 项目名，1 = 文件名。 */
function dialogInputs(wrapper: ReturnType<typeof mount>) {
  return wrapper.findAll("input");
}

function findButton(wrapper: ReturnType<typeof mount>, label: string) {
  const found = wrapper.findAll("button").find((candidate) => candidate.text() === label);
  if (found === undefined) {
    throw new Error(`找不到按钮：${label}`);
  }
  return found;
}

// 对话框 Teleport 到 body：不卸载会跨用例残留；stubs 让内容留在 wrapper 内便于断言。
enableAutoUnmount(afterEach);

describe("NewProjectDialog", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.resetAllMocks();
    useNewProjectDialog().hide();
  });

  it("收起态不渲染；show 后打开且字段清空", async () => {
    const wrapper = mount(NewProjectDialog, {
      global: { plugins: [pinia], stubs: { teleport: true } },
    });
    expect(wrapper.find("input").exists()).toBe(false);

    useNewProjectDialog().show();
    await wrapper.vm.$nextTick();
    const inputs = dialogInputs(wrapper);
    expect(inputs).toHaveLength(2);
    expect(inputs[0]?.element.value).toBe("");
    expect(inputs[1]?.element.value).toBe("");
  });

  it("文件名默认跟随项目名，手动改过之后不再跟随", async () => {
    const { createProject } = await import("../../../src-web/api/project");
    vi.mocked(createProject).mockResolvedValue(makeProject("支架") as never);
    useNewProjectDialog().show();

    const wrapper = mount(NewProjectDialog, {
      global: { plugins: [pinia], stubs: { teleport: true } },
    });
    await dialogInputs(wrapper)[0]?.setValue("支架");
    expect(dialogInputs(wrapper)[1]?.element.value).toBe("支架");

    // 手动改文件名后，再改项目名不再覆盖文件名。
    await dialogInputs(wrapper)[1]?.setValue("控制器支架-v2");
    await dialogInputs(wrapper)[0]?.setValue("支架 B");
    expect(dialogInputs(wrapper)[1]?.element.value).toBe("控制器支架-v2");
  });

  it("项目名为空时提示且不建项目", async () => {
    const wrapper = mount(NewProjectDialog, {
      global: { plugins: [pinia], stubs: { teleport: true } },
    });
    useNewProjectDialog().show();
    await wrapper.vm.$nextTick();

    await dialogInputs(wrapper)[0]?.setValue("   ");
    await findButton(wrapper, "创建").trigger("click");
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("项目名不能为空。");
    expect(useProjectStore().project).toBeNull();
    expect(useNewProjectDialog().open.value).toBe(true);
  });

  it("创建成功后收起对话框，路径用项目名与文件名", async () => {
    const { createProject } = await import("../../../src-web/api/project");
    vi.mocked(createProject).mockResolvedValue(makeProject("控制器支架") as never);

    useNewProjectDialog().show();
    const wrapper = mount(NewProjectDialog, {
      global: { plugins: [pinia], stubs: { teleport: true } },
    });
    await dialogInputs(wrapper)[0]?.setValue("控制器支架");
    await dialogInputs(wrapper)[1]?.setValue("v2 试模");
    await findButton(wrapper, "创建").trigger("click");
    await vi.waitFor(() => expect(useNewProjectDialog().open.value).toBe(false));

    expect(createProject).toHaveBeenCalledWith("控制器支架");
    const project = useProjectStore();
    expect(project.project?.name).toBe("控制器支架");
    expect(project.projectPath).toBe("/home/u/Documents/kairos/控制器支架/v2 试模.kairos");
  });

  it("文件名为空时落到项目名；创建失败保持打开", async () => {
    const { createProject } = await import("../../../src-web/api/project");
    vi.mocked(createProject).mockResolvedValue(makeProject("支架") as never);

    useNewProjectDialog().show();
    const wrapper = mount(NewProjectDialog, {
      global: { plugins: [pinia], stubs: { teleport: true } },
    });
    await dialogInputs(wrapper)[0]?.setValue("支架");
    // 用户清空文件名：落回项目名（字段默认跟随，清空后由提交兜底）。
    await dialogInputs(wrapper)[1]?.setValue("");
    await findButton(wrapper, "创建").trigger("click");
    await vi.waitFor(() => expect(useNewProjectDialog().open.value).toBe(false));
    expect(useProjectStore().projectPath).toBe("/home/u/Documents/kairos/支架/支架.kairos");

    // 失败：对话框保持打开（错误经全局通道呈现）。
    useNewProjectDialog().show();
    await wrapper.vm.$nextTick();
    vi.mocked(createProject).mockRejectedValueOnce(new Error("磁盘不可写"));
    await dialogInputs(wrapper)[0]?.setValue("支架二");
    await findButton(wrapper, "创建").trigger("click");
    await vi.waitFor(() => expect(useAppStore().error?.message).toBe("磁盘不可写"));
    expect(useNewProjectDialog().open.value).toBe(true);
  });

  it("取消 / 点击遮罩关闭", async () => {
    useNewProjectDialog().show();
    const wrapper = mount(NewProjectDialog, {
      global: { plugins: [pinia], stubs: { teleport: true } },
    });

    await findButton(wrapper, "取消").trigger("click");
    expect(useNewProjectDialog().open.value).toBe(false);

    useNewProjectDialog().show();
    await wrapper.vm.$nextTick();
    await wrapper.find(".fixed").trigger("click");
    expect(useNewProjectDialog().open.value).toBe(false);
  });
});
