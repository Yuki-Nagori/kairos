import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { enableAutoUnmount, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import NewProjectDialog from "../../../src-web/components/menu-bar/NewProjectDialog.vue";
import { useNewProjectDialog } from "../../../src-web/components/menu-bar/useNewProjectDialog";
import { useProjectStore } from "../../../src-web/stores/project";
import { useAppStore } from "../../../src-web/stores/app";
import { defaultWorkspacePath, projectPath } from "../../../src-web/api/project";
import { pickWorkspaceDir } from "../../../src-web/api/dialog";

vi.mock("../../../src-web/api/project", () => ({
  createProject: vi.fn(),
  resetProjectSession: vi.fn(),
  defaultWorkspacePath: vi.fn(async () => "/home/u/Documents/kairos"),
  projectPath: vi.fn(
    async (workspace: string, name: string) => `${workspace}/${name}/${name}.kairos`,
  ),
  workspaceRootOf: vi.fn(async (path: string) => path.split("/").slice(0, -2).join("/") || null),
  archiveWorkspaceGeometry: vi.fn(),
  loadWorkspaceGeometry: vi.fn(),
  saveStudyMesh: vi.fn(),
  restoreStudyMesh: vi.fn(),
  listRecentProjects: vi.fn(async () => []),
  loadProjectFile: vi.fn(),
  saveProjectFile: vi.fn(async () => undefined),
}));
vi.mock("../../../src-web/api/dialog", () => ({
  pickWorkspaceDir: vi.fn(),
  pickOpenProjectPath: vi.fn(),
  pickSaveProjectPath: vi.fn(),
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

/** 输入框顺序：0 = 工作区路径，1 = 项目名。 */
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
    vi.mocked(defaultWorkspacePath).mockResolvedValue("/home/u/Documents/kairos");
    vi.mocked(projectPath).mockImplementation(
      async (workspace: string, name: string) => `${workspace}/${name}/${name}.kairos`,
    );
    useNewProjectDialog().hide();
  });

  it("show 打开对话框：工作区初值取平台默认根，项目名清空", async () => {
    const wrapper = mount(NewProjectDialog, {
      global: { plugins: [pinia], stubs: { teleport: true } },
    });
    expect(wrapper.find("input").exists()).toBe(false);

    await useNewProjectDialog().show();
    await wrapper.vm.$nextTick();
    const inputs = dialogInputs(wrapper);
    expect(inputs).toHaveLength(2);
    expect(inputs[0]?.element.value).toBe("/home/u/Documents/kairos");
    expect(inputs[1]?.element.value).toBe("");
  });

  it("取不到默认工作区时保留上次值（IPC 不可用）", async () => {
    vi.mocked(defaultWorkspacePath).mockRejectedValueOnce(new Error("IPC 不可用"));
    const { workspace } = useNewProjectDialog();
    workspace.value = "/Volumes/Work";

    await useNewProjectDialog().show();
    expect(workspace.value).toBe("/Volumes/Work");
  });

  it("落点预览随工作区与项目名更新", async () => {
    const wrapper = mount(NewProjectDialog, {
      global: { plugins: [pinia], stubs: { teleport: true } },
    });
    await useNewProjectDialog().show();
    await wrapper.vm.$nextTick();
    await dialogInputs(wrapper)[0]?.setValue("/Volumes/Work/kairos/");
    await dialogInputs(wrapper)[1]?.setValue("控制器支架");
    expect(wrapper.text()).toContain("/Volumes/Work/kairos/控制器支架/控制器支架.kairos");
  });

  it("目录选择器失败时记全局错误且不改工作区", async () => {
    const app = (await import("../../../src-web/stores/app")).useAppStore();
    const { workspace } = useNewProjectDialog();
    workspace.value = "/Volumes/Work";
    vi.mocked(pickWorkspaceDir).mockRejectedValueOnce(new Error("对话框不可用"));

    await useNewProjectDialog().browseWorkspace();
    expect(app.error?.message).toBe("对话框不可用");
    expect(workspace.value).toBe("/Volumes/Work");
  });

  it("「选择…」用系统目录选择器改工作区；取消保持原值", async () => {
    const wrapper = mount(NewProjectDialog, {
      global: { plugins: [pinia], stubs: { teleport: true } },
    });
    await useNewProjectDialog().show();
    await wrapper.vm.$nextTick();

    vi.mocked(pickWorkspaceDir).mockResolvedValueOnce("/Volumes/Work/kairos");
    await findButton(wrapper, "选择…").trigger("click");
    await vi.waitFor(() =>
      expect(dialogInputs(wrapper)[0]?.element.value).toBe("/Volumes/Work/kairos"),
    );

    vi.mocked(pickWorkspaceDir).mockResolvedValueOnce(null);
    await findButton(wrapper, "选择…").trigger("click");
    await wrapper.vm.$nextTick();
    expect(dialogInputs(wrapper)[0]?.element.value).toBe("/Volumes/Work/kairos");
  });

  it("项目名为空时提示且不建项目", async () => {
    const wrapper = mount(NewProjectDialog, {
      global: { plugins: [pinia], stubs: { teleport: true } },
    });
    await useNewProjectDialog().show();
    await wrapper.vm.$nextTick();

    await dialogInputs(wrapper)[1]?.setValue("   ");
    await findButton(wrapper, "创建").trigger("click");
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("项目名不能为空。");
    expect(useProjectStore().project).toBeNull();
    expect(useNewProjectDialog().open.value).toBe(true);
  });

  it("创建成功后收起对话框：工程文件与项目名同名", async () => {
    const { createProject } = await import("../../../src-web/api/project");
    vi.mocked(createProject).mockResolvedValue(makeProject("控制器支架") as never);

    const wrapper = mount(NewProjectDialog, {
      global: { plugins: [pinia], stubs: { teleport: true } },
    });
    await useNewProjectDialog().show();
    await wrapper.vm.$nextTick();
    await dialogInputs(wrapper)[0]?.setValue("/Volumes/Work");
    await dialogInputs(wrapper)[1]?.setValue("控制器支架");
    await findButton(wrapper, "创建").trigger("click");
    await vi.waitFor(() => expect(useNewProjectDialog().open.value).toBe(false));

    expect(projectPath).toHaveBeenCalledWith("/Volumes/Work", "控制器支架");
    const project = useProjectStore();
    expect(project.project?.name).toBe("控制器支架");
    expect(project.projectPath).toBe("/Volumes/Work/控制器支架/控制器支架.kairos");
  });

  it("创建失败保持打开（错误经全局通道呈现）", async () => {
    const { createProject } = await import("../../../src-web/api/project");
    vi.mocked(createProject).mockRejectedValue(new Error("工作区不可写"));

    const wrapper = mount(NewProjectDialog, {
      global: { plugins: [pinia], stubs: { teleport: true } },
    });
    await useNewProjectDialog().show();
    await wrapper.vm.$nextTick();
    await dialogInputs(wrapper)[1]?.setValue("支架");
    await findButton(wrapper, "创建").trigger("click");

    await vi.waitFor(() => expect(useAppStore().error?.message).toBe("工作区不可写"));
    expect(useNewProjectDialog().open.value).toBe(true);
  });

  it("取消 / 点击遮罩关闭", async () => {
    const wrapper = mount(NewProjectDialog, {
      global: { plugins: [pinia], stubs: { teleport: true } },
    });
    await useNewProjectDialog().show();
    await wrapper.vm.$nextTick();

    await findButton(wrapper, "取消").trigger("click");
    expect(useNewProjectDialog().open.value).toBe(false);

    await useNewProjectDialog().show();
    await wrapper.vm.$nextTick();
    await wrapper.find(".fixed").trigger("click");
    expect(useNewProjectDialog().open.value).toBe(false);
  });
});
