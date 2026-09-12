/** 阶段选项卡测试：tab 渲染、点击切换阶段、方案任务状态角标投射。 */
import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import StageTabs from "../../../../src-web/components/stage-tabs/StageTabs.vue";
import { useAppStore } from "../../../../src-web/stores/app";
import { useGeometryStore } from "../../../../src-web/stores/geometry";
import { useJobsStore } from "../../../../src-web/stores/jobs";

vi.mock("../../../../src-web/api/geometry", () => ({ getRenderMesh: vi.fn() }));
vi.mock("../../../../src-web/api/results", () => ({
  listResultTimes: vi.fn(),
  loadResultField: vi.fn(),
  deriveField: vi.fn(),
  deriveDifference: vi.fn(),
}));
vi.mock("../../../../src-web/api/project", () => ({
  createProject: vi.fn(),
  listRecentProjects: vi.fn(),
  loadProjectFile: vi.fn(),
  saveProjectFile: vi.fn(),
}));
vi.mock("../../../../src-web/api/materials", () => ({
  listBuiltinMaterials: vi.fn(async () => []),
  listCustomMaterials: vi.fn(async () => []),
  importCustomMaterials: vi.fn(),
  upsertCustomMaterial: vi.fn(),
  deleteCustomMaterial: vi.fn(),
  exportMaterialsToFile: vi.fn(),
}));

describe("StageTabs", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("按工作流顺序渲染七个阶段选项卡", () => {
    const wrapper = mount(StageTabs, { global: { plugins: [pinia] } });
    const labels = wrapper.findAll("button").map((node) => node.text());
    expect(labels.map((label) => label.replace(/[!⟳⧖✕]/g, "").trim())).toEqual([
      "主页",
      "几何",
      "网格",
      "工艺",
      "求解",
      "结果",
      "报告",
    ]);
  });

  it("点击选项卡切换分析阶段", async () => {
    const app = useAppStore();
    const wrapper = mount(StageTabs, { global: { plugins: [pinia] } });
    await wrapper.findAll("button")[3]!.trigger("click");
    expect(app.stage).toBe("process");
  });

  it("几何健康告警 → 几何选项卡渲染 ! 角标", async () => {
    const geometry = useGeometryStore();
    geometry.geometries = [
      {
        geometryId: "g-1",
        fileName: "mug.stl",
        triangleCount: 100,
        size: [10, 10, 10],
        surfaceArea: 600,
        signedVolume: 1000,
        suggestedUnit: "mm",
        issues: { degenerate: 1, openEdges: 2, nonManifoldEdges: 0, normalInconsistentEdges: 0 },
      },
    ];
    const wrapper = mount(StageTabs, { global: { plugins: [pinia] } });
    const geometryTab = wrapper.findAll("button")[1]!;
    expect(geometryTab.text()).toContain("!");
    expect(geometryTab.find("span[title]").attributes("title")).toContain("告警");
  });

  it("作业运行 / 失败 → 求解选项卡渲染 ⟳ / ✕ 角标", async () => {
    const jobsStore = useJobsStore();
    jobsStore.jobs = [
      {
        id: "job-1",
        studyId: null,
        caseDir: "/case",
        cores: 2,
        status: "running",
        createdMs: 1,
        startedMs: 1,
        finishedMs: null,
        lastTimeS: 0.5,
        message: null,
      },
    ];
    const wrapper = mount(StageTabs, { global: { plugins: [pinia] } });
    const solveTab = wrapper.findAll("button")[4]!;
    expect(solveTab.text()).toContain("⟳");

    jobsStore.jobs = [
      {
        id: "job-1",
        studyId: null,
        caseDir: "/case",
        cores: 2,
        status: "failed",
        createdMs: 1,
        startedMs: 1,
        finishedMs: 2,
        lastTimeS: null,
        message: null,
      },
    ];
    await wrapper.vm.$nextTick();
    expect(wrapper.findAll("button")[4]!.text()).toContain("✕");
  });

  it("空闲状态无任何角标", () => {
    const wrapper = mount(StageTabs, { global: { plugins: [pinia] } });
    for (const tab of wrapper.findAll("button")) {
      expect(tab.find("span[title]").exists()).toBe(false);
    }
  });
});
