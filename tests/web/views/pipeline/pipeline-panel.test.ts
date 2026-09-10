import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import PipelinePanel from "../../../../src-web/views/pipeline/PipelinePanel.vue";
import { useAppStore } from "../../../../src-web/stores/app";
import { useGeometryStore } from "../../../../src-web/stores/geometry";
import { useJobsStore } from "../../../../src-web/stores/jobs";
import { useMaterialsStore } from "../../../../src-web/stores/materials";
import { useProjectStore } from "../../../../src-web/stores/project";
import { defaultCaseDir } from "../../../../src-web/api/project";
import { generateOpenfoamCase } from "../../../../src-web/api/solver";
import { listJobs, submitJob } from "../../../../src-web/api/jobs";
import type {
  GeometrySummary,
  Job,
  Material,
  MeshingReport,
  ProcessSettings,
  Project,
  Study,
} from "../../../../src-web/types";

// happy-dom 无 Tauri IPC：Channel 桩为可赋值 onmessage 的最小实现。
vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: ((line: string) => void) | null = null;
  },
}));

vi.mock("../../../../src-web/api/project", () => ({
  createProject: vi.fn(),
  listRecentProjects: vi.fn(),
  loadProjectFile: vi.fn(),
  saveProjectFile: vi.fn(),
  defaultCaseDir: vi.fn(),
}));
vi.mock("../../../../src-web/api/solver", () => ({
  generateOpenfoamCase: vi.fn(),
  probeOpenfoam: vi.fn(),
}));
vi.mock("../../../../src-web/api/jobs", () => ({
  submitJob: vi.fn(),
  listJobs: vi.fn(),
  cancelJob: vi.fn(),
}));

function materialFixture(): Material {
  return {
    id: "mat-1",
    name: "PP-示例",
    manufacturer: "示例数据",
    family: "PP",
    rheology: { n: 0.32, tauStar: 2e4, d1: 1.1e13, d2: 263, d3: 0, a1: 31, a2: 51.6 },
    pvt: {
      b1m: 1.28e-3,
      b1s: 1.22e-3,
      b2m: 7.5e-7,
      b2s: 3e-7,
      b3: 1.4e8,
      b4m: 3e-3,
      b4s: 1.5e-3,
      b5: 418,
    },
    specificHeat: [[300, 1900]],
    conductivity: [[300, 0.22]],
    mechanics: null,
    dataNote: "示例数据",
  };
}

function processFixture(): ProcessSettings {
  return {
    meltTempC: 230,
    moldTempC: 40,
    ejectionTempC: 90,
    injectionTimeS: 1.5,
    vpSwitchVolumePercent: 96,
    packingPressureMpaCurve: [
      [0, 60],
      [8, 48],
    ],
    packingTimeS: 8,
    coolingTimeS: 15,
    coolantTempC: 25,
  };
}

function geometryFixture(): GeometrySummary {
  return {
    geometryId: "geo-1",
    fileName: "demo.stl",
    triangleCount: 12,
    size: [10, 20, 30],
    surfaceArea: 2200,
    signedVolume: 6000,
    suggestedUnit: "mm",
    issues: { degenerate: 0, openEdges: 0, nonManifoldEdges: 0, normalInconsistentEdges: 0 },
  };
}

function meshReportFixture(): MeshingReport {
  return {
    engine: "voxel",
    nodeCount: 8,
    elementCount: 12,
    surfaceFaceCount: 6,
    totalVolume: 1000,
    quality: { minEdgeRatio: 0.7, avgEdgeRatio: 0.85, maxEdgeRatio: 0.99, minVolume: 1 },
  };
}

function studyFixture(): Study {
  return {
    id: "study-1",
    name: "填充研究",
    createdMs: 1,
    runnerElements: [],
    coolingChannels: [],
    process: processFixture(),
    materialId: "mat-1",
  };
}

function projectFixture(studies: Study[]): Project {
  return {
    schemaVersion: 1,
    id: "p-1",
    name: "演示项目",
    createdMs: 1,
    updatedMs: 1,
    studies,
  };
}

function jobFixture(overrides: Partial<Job> = {}): Job {
  return {
    id: "job-1",
    studyId: "study-1",
    caseDir: "/case",
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

function findButton(wrapper: ReturnType<typeof mount>, label: string) {
  const found = wrapper.findAll("button").find((candidate) => candidate.text() === label);
  if (found === undefined) {
    throw new Error(`找不到按钮：${label}`);
  }
  return found;
}

/** 全前置就绪的完整状态：几何 + 网格 + 材料 + 工艺的研究。 */
function setupReadyState(): void {
  const geometry = useGeometryStore();
  const materials = useMaterialsStore();
  const project = useProjectStore();
  geometry.geometries = [geometryFixture()];
  geometry.meshReports = { "geo-1": meshReportFixture() };
  materials.materials = { builtin: [materialFixture()], custom: [] };
  project.project = projectFixture([studyFixture()]);
  project.activeStudyId = "study-1";
}

describe("PipelinePanel", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.resetAllMocks();
    vi.mocked(defaultCaseDir).mockResolvedValue("/case/study-1");
    vi.mocked(generateOpenfoamCase).mockResolvedValue("/case/study-1");
    vi.mocked(submitJob).mockResolvedValue(jobFixture());
    vi.mocked(listJobs).mockResolvedValue([]);
  });

  it("空状态：五步全部未完成，渲染编号与指引，提交按钮禁用", () => {
    const wrapper = mount(PipelinePanel, { global: { plugins: [pinia] } });
    const text = wrapper.text();
    expect(text).toContain("导入几何（STL 或样例）");
    expect(text).toContain("在几何面板点击「导入样例」或导入 STL 文件。");
    expect(text).toContain("先完成几何导入。");
    expect(text).toContain("在材料库面板选择材料并点击「用于当前研究」。");
    expect(text).toContain("在工艺设置面板填写参数并「校验并应用到研究」。");
    expect(text).toContain("全部就绪后点击「提交求解作业」。");
    expect(findButton(wrapper, "提交求解作业").attributes("disabled")).toBeDefined();
    // 空闲时实时状态行占位为空。
    expect(wrapper.find(".tabular-nums").text()).toBe("");
  });

  it("几何已导入但未划分网格：首步打勾，网格步给出对应指引", () => {
    const geometry = useGeometryStore();
    geometry.geometries = [geometryFixture()];

    const wrapper = mount(PipelinePanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("✓");
    expect(wrapper.text()).toContain("在几何面板设置目标尺寸并点击「生成体积网格」。");
    expect(findButton(wrapper, "提交求解作业").attributes("disabled")).toBeDefined();
  });

  it("前置全就绪时按钮启用，完成步骤的指引消失", () => {
    setupReadyState();
    const wrapper = mount(PipelinePanel, { global: { plugins: [pinia] } });
    expect(findButton(wrapper, "提交求解作业").attributes("disabled")).toBeUndefined();
    // 前四步完成，仅剩「提交求解作业」一步带指引。
    expect(wrapper.text()).not.toContain("在几何面板点击「导入样例」或导入 STL 文件。");
    expect(wrapper.text()).not.toContain("在材料库面板选择材料并点击「用于当前研究」。");
    expect(wrapper.text()).not.toContain("在工艺设置面板填写参数并「校验并应用到研究」。");
  });

  it("应用忙碌时按钮禁用（即使前置全就绪）", async () => {
    setupReadyState();
    const app = useAppStore();
    app.beginBusy("正在提交作业…");
    const wrapper = mount(PipelinePanel, { global: { plugins: [pinia] } });
    expect(findButton(wrapper, "提交求解作业").attributes("disabled")).toBeDefined();
    app.endBusy();
    await nextTick();
    expect(findButton(wrapper, "提交求解作业").attributes("disabled")).toBeUndefined();
  });

  it("运行中的作业实时显示物理时间；时间缺失或已完成时行占位为空", async () => {
    const jobsStore = useJobsStore();
    const wrapper = mount(PipelinePanel, { global: { plugins: [pinia] } });

    jobsStore.jobs = [jobFixture({ status: "running", lastTimeS: 1.234 })];
    await nextTick();
    expect(wrapper.find(".tabular-nums").text()).toBe("⟳ 求解中 · T = 1.23 s");

    jobsStore.jobs = [jobFixture({ status: "running", lastTimeS: null })];
    await nextTick();
    expect(wrapper.find(".tabular-nums").text()).toBe("");

    // 已完成的作业让「提交求解作业」步骤视为完成。
    jobsStore.jobs = [jobFixture({ status: "done" })];
    await nextTick();
    expect(wrapper.find(".tabular-nums").text()).toBe("");
    expect(wrapper.text()).not.toContain("全部就绪后点击「提交求解作业」。");
  });

  it("提交成功：核数与阶段透传到 case 生成与作业入队", async () => {
    setupReadyState();
    const app = useAppStore();
    const wrapper = mount(PipelinePanel, { global: { plugins: [pinia] } });

    await wrapper.find("select").setValue("fill_pack");
    await wrapper.find("input").setValue("4");
    await findButton(wrapper, "提交求解作业").trigger("click");
    await flushPromises();

    expect(defaultCaseDir).toHaveBeenCalledWith("study-1");
    expect(generateOpenfoamCase).toHaveBeenCalledWith(
      expect.objectContaining({
        geometryId: "geo-1",
        caseDir: "/case/study-1",
        stage: "fill_pack",
        cores: 4,
        material: expect.objectContaining({ id: "mat-1" }),
        process: expect.objectContaining({ meltTempC: 230 }),
      }),
    );
    expect(submitJob).toHaveBeenCalledWith("/case/study-1", 4, "study-1", expect.anything());
    expect(app.error).toBeNull();
    expect(app.busy).toBeNull();
  });

  it("核数留空按 2 核提交", async () => {
    setupReadyState();
    const wrapper = mount(PipelinePanel, { global: { plugins: [pinia] } });

    await findButton(wrapper, "提交求解作业").trigger("click");
    await flushPromises();

    expect(submitJob).toHaveBeenCalledWith("/case/study-1", 2, "study-1", expect.anything());
  });

  it("case 生成失败：错误进入全局状态，不入队作业，忙碌复位", async () => {
    setupReadyState();
    vi.mocked(generateOpenfoamCase).mockRejectedValue(new Error("磁盘不可写"));
    const app = useAppStore();
    const wrapper = mount(PipelinePanel, { global: { plugins: [pinia] } });

    await findButton(wrapper, "提交求解作业").trigger("click");
    await flushPromises();

    expect(app.error?.message).toBe("磁盘不可写");
    expect(app.busy).toBeNull();
    expect(submitJob).not.toHaveBeenCalled();
  });
});
