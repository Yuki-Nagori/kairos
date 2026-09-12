/** 方案任务窗格测试：Moldflow 式任务序列渲染、双击导航、提交门控。 */
import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import StudyTasksPanel from "../../../../src-web/views/study-tasks/StudyTasksPanel.vue";
import { useGeometryStore } from "../../../../src-web/stores/geometry";
import { useJobsStore } from "../../../../src-web/stores/jobs";
import { useMaterialsStore } from "../../../../src-web/stores/materials";
import { useProjectStore } from "../../../../src-web/stores/project";
import { useResultsStore } from "../../../../src-web/stores/results";
import type { Mock } from "vitest";
import type { Job, Project, ResultCatalog } from "../../../../src-web/types";

vi.mock("../../../../src-web/api/results", () => ({
  listResultTimes: vi.fn(),
  loadResultField: vi.fn(),
  deriveField: vi.fn(),
  deriveDifference: vi.fn(),
}));
vi.mock("../../../../src-web/api/solver", () => ({
  generateMoldingfoamCase: vi.fn(),
  probeMoldingfoam: vi.fn(),
}));
vi.mock("../../../../src-web/api/project", () => ({
  defaultCaseDir: vi.fn(async () => "/case/run"),
}));
vi.mock("../../../../src-web/api/jobs", () => ({
  submitJob: vi.fn(),
  cancelJob: vi.fn(),
  // refreshJobs 会把返回值写入 store.jobs：必须返回数组，否则污染任务快照
  listJobs: vi.fn(async () => []),
}));
// Channel 在 happy-dom 无 Tauri IPC 内部对象，构造即抛——桩掉
vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: ((line: string) => void) | null = null;
  },
}));

function projectFixture(): Project {
  return {
    schemaVersion: 4,
    id: "p-1",
    name: "演示",
    createdMs: 1,
    updatedMs: 1,
    studies: [
      {
        id: "s-1",
        name: "填充分析",
        createdMs: 1,
        runnerElements: [],
        coolingChannels: [],
        process: {
          meltTempC: 230,
          moldTempC: 40,
          ejectionTempC: 90,
          injectionTimeS: 1,
          vpSwitchVolumePercent: 96,
          packingPressureMpaCurve: [[0, 60]],
          packingTimeS: 8,
          coolingTimeS: 15,
          coolantTempC: 25,
        },
        materialId: "m-1",
      },
    ],
  };
}

function jobFixture(overrides: Partial<Job>): Job {
  return {
    id: "job-1",
    studyId: "s-1",
    caseDir: "/case/run",
    cores: 2,
    status: "queued",
    createdMs: 1,
    startedMs: null,
    finishedMs: null,
    lastTimeS: null,
    message: null,
    ...overrides,
  };
}

function geometryFixture() {
  return {
    geometryId: "g-1",
    fileName: "mug.stl",
    triangleCount: 1200,
    size: [20, 20, 30] as [number, number, number],
    surfaceArea: 3000,
    signedVolume: 8000,
    suggestedUnit: "mm",
    issues: { degenerate: 0, openEdges: 0, nonManifoldEdges: 0, normalInconsistentEdges: 0 },
  };
}

function materialFixture() {
  return {
    id: "m-1",
    name: "PP-REF-01",
    family: "PP",
    manufacturer: "示例石化",
    rheology: {} as never,
    pvt: {} as never,
    specificHeat: [[200, 2000]] as [number, number][],
    thermalConductivity: [[200, 0.2]] as [number, number][],
    mechanics: null,
    filler: null,
    dataNote: "",
  };
}

function prime(_pinia: Pinia): void {
  const geometry = useGeometryStore();
  const project = useProjectStore();
  const materials = useMaterialsStore();
  materials.materials = { builtin: [materialFixture() as never], custom: [] };
  geometry.geometries = [geometryFixture()];
  geometry.meshReports["g-1"] = {
    engine: "voxel",
    nodeCount: 500,
    elementCount: 5000,
    surfaceFaceCount: 1000,
    totalVolume: 8000,
    quality: { minEdgeRatio: 0.4, avgEdgeRatio: 0.8, maxEdgeRatio: 1.2, minVolume: 0.01 },
  };
  project.project = projectFixture();
  project.activeStudyId = "s-1";
}

describe("StudyTasksPanel（方案任务窗格）", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("任务序列按执行顺序渲染，完成态带摘要", () => {
    prime(pinia);
    const wrapper = mount(StudyTasksPanel, { global: { plugins: [pinia] } });
    const labels = wrapper.findAll("ol li span").map((node) => node.text());
    expect(labels).toContain("导入几何");
    expect(labels).toContain("网格划分");
    expect(labels).toContain("选择材料");
    expect(labels).toContain("成型工艺设置");
    expect(labels).toContain("分析（填充 / 保压）");
    expect(labels).toContain("结果分析");
    // 修复任务仅在几何不健康时出现
    expect(labels).not.toContain("修复网格");
    // 完成摘要
    expect(wrapper.text()).toContain("四面体 5000");
  });

  it("双击任务切换到对应工作台阶段（Moldflow 双击打开编辑器）", async () => {
    prime(pinia);
    const app = (await import("../../../../src-web/stores/app")).useAppStore();
    const wrapper = mount(StudyTasksPanel, { global: { plugins: [pinia] } });
    const buttons = wrapper.findAll("ol li button");

    // 第一个任务 = 导入几何 → geometry 阶段
    await buttons[0]!.trigger("dblclick");
    expect(app.stage).toBe("geometry");
    // 分析任务（倒数第二个）→ solve 阶段
    await buttons[buttons.length - 2]!.trigger("dblclick");
    expect(app.stage).toBe("solve");
  });

  it("分析作业失败 → 结果任务显示阻断原因；✕ 图标出现", async () => {
    prime(pinia);
    const jobs = useJobsStore();
    jobs.jobs = [jobFixture({ status: "failed", message: "foamRun 崩溃" })];
    const wrapper = mount(StudyTasksPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("分析作业失败");
    expect(wrapper.text()).toContain("✕");
  });

  it("右键任务弹出上下文菜单：打开编辑恒有，Esc / 点击外部关闭", async () => {
    prime(pinia);
    const app = (await import("../../../../src-web/stores/app")).useAppStore();
    const wrapper = mount(StudyTasksPanel, { global: { plugins: [pinia] } });
    const buttons = wrapper.findAll("ol li button");

    // 右键几何任务 → 菜单出现，含「打开编辑」
    await buttons[0]!.trigger("contextmenu", { clientX: 40, clientY: 60 });
    const menu = wrapper.find("div.fixed");
    expect(menu.exists()).toBe(true);
    expect(menu.text()).toContain("打开编辑");
    const labels = menu.findAll("button").map((node) => node.text());
    expect(labels).toEqual(["打开编辑"]);

    // 点击菜单项 → 切换阶段并关闭菜单
    await menu.findAll("button")[0]!.trigger("click");
    expect(app.stage).toBe("geometry");
    expect(wrapper.find("div.fixed").exists()).toBe(false);
  });

  it("右键分析任务：运行中出现「取消作业」并调用取消；结果任务已扫描出现「重扫」", async () => {
    prime(pinia);
    const jobsStore = useJobsStore();
    const results = useResultsStore();
    const { cancelJob } = (await import("../../../../src-web/api/jobs")) as unknown as {
      cancelJob: ReturnType<typeof vi.fn>;
    };
    jobsStore.jobs = [jobFixture({ status: "running", lastTimeS: 0.5 })];
    const wrapper = mount(StudyTasksPanel, { global: { plugins: [pinia] } });
    const buttons = wrapper.findAll("ol li button");

    // 分析任务（倒数第二）右键 → 取消作业
    await buttons[buttons.length - 2]!.trigger("contextmenu", { clientX: 10, clientY: 10 });
    let menu = wrapper.find("div.fixed");
    expect(menu.text()).toContain("取消作业");
    await menu
      .findAll("button")
      .find((node) => node.text() === "取消作业")!
      .trigger("click");
    await flushPromises();
    expect(cancelJob).toHaveBeenCalledWith("job-1");
    expect(wrapper.find("div.fixed").exists()).toBe(false);

    // 结果任务：已扫描 → 重扫结果目录
    results.resultCatalog = {
      caseDir: "/case/run",
      times: [{ dirName: "1", timeS: 1, fields: ["p"] }],
    };
    await wrapper.vm.$nextTick();
    const rescanSpy = vi.spyOn(results, "rescanCatalog").mockResolvedValue(undefined);
    await buttons[buttons.length - 1]!.trigger("contextmenu", { clientX: 10, clientY: 10 });
    menu = wrapper.find("div.fixed");
    expect(menu.text()).toContain("重扫结果目录");
    await menu
      .findAll("button")
      .find((node) => node.text() === "重扫结果目录")!
      .trigger("click");
    expect(rescanSpy).toHaveBeenCalledTimes(1);
  });

  it("Esc 关闭右键菜单", async () => {
    prime(pinia);
    const wrapper = mount(StudyTasksPanel, { global: { plugins: [pinia] } });
    await wrapper.findAll("ol li button")[0]!.trigger("contextmenu", { clientX: 1, clientY: 1 });
    expect(wrapper.find("div.fixed").exists()).toBe(true);

    // 非 Esc 按键不关闭菜单
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
    await wrapper.vm.$nextTick();
    expect(wrapper.find("div.fixed").exists()).toBe(true);

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await wrapper.vm.$nextTick();
    expect(wrapper.find("div.fixed").exists()).toBe(false);
    wrapper.unmount();
  });

  it("分析待办时右键：只有「打开编辑」（无作业可取消）", async () => {
    prime(pinia);
    const wrapper = mount(StudyTasksPanel, { global: { plugins: [pinia] } });
    const buttons = wrapper.findAll("ol li button");
    await buttons[buttons.length - 2]!.trigger("contextmenu", { clientX: 1, clientY: 1 });
    const labels = wrapper
      .find("div.fixed")
      .findAll("button")
      .map((node) => node.text());
    expect(labels).toEqual(["打开编辑"]);
    wrapper.unmount();
  });

  it("分析排队中右键：取消作业同样可用（⧖ 态）", async () => {
    prime(pinia);
    const jobsStore = useJobsStore();
    jobsStore.jobs = [jobFixture({ status: "queued" })];
    const wrapper = mount(StudyTasksPanel, { global: { plugins: [pinia] } });
    const buttons = wrapper.findAll("ol li button");
    await buttons[buttons.length - 2]!.trigger("contextmenu", { clientX: 1, clientY: 1 });
    const labels = wrapper
      .find("div.fixed")
      .findAll("button")
      .map((node) => node.text());
    expect(labels).toEqual(["打开编辑", "取消作业"]);
    wrapper.unmount();
  });

  it("前置未就绪时提交按钮禁用；就绪后可用并调用提交", async () => {
    prime(pinia);
    const results = useResultsStore();
    results.resultCatalog = {
      caseDir: "/case/run",
      times: [{ dirName: "1", timeS: 1, fields: ["p"] }],
    } as ResultCatalog;
    const wrapper = mount(StudyTasksPanel, { global: { plugins: [pinia] } });
    const submit = wrapper.findAll("button").find((node) => node.text() === "提交求解作业")!;
    expect(submit.attributes("disabled")).toBeUndefined();

    // 前置破坏（清空几何）→ 禁用
    const geometry = useGeometryStore();
    geometry.geometries = [];
    await flushPromises();
    expect(submit.attributes("disabled")).toBeDefined();

    // 恢复前置 → 点击提交：走 pipeline store 端到端（case 生成 + 作业入队）
    geometry.geometries = [geometryFixture()];
    await flushPromises();
    await submit.trigger("click");
    await flushPromises();
    const { generateMoldingfoamCase } =
      (await import("../../../../src-web/api/solver")) as unknown as {
        generateMoldingfoamCase: Mock;
      };
    const { submitJob } = (await import("../../../../src-web/api/jobs")) as unknown as {
      submitJob: Mock;
    };
    expect(generateMoldingfoamCase).toHaveBeenCalledTimes(1);
    expect(submitJob).toHaveBeenCalledTimes(1);
  });
});
