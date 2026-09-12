/** 方案摘要面板测试：材料 / 工艺 / 最新作业三段速览与空态占位。 */
import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import StudySummaryPanel from "../../../../src-web/views/study-summary/StudySummaryPanel.vue";
import { useJobsStore } from "../../../../src-web/stores/jobs";
import { useMaterialsStore } from "../../../../src-web/stores/materials";
import { useProjectStore } from "../../../../src-web/stores/project";
import type { Job, Material, Project } from "../../../../src-web/types";

vi.mock("../../../../src-web/api/project", () => ({
  createProject: vi.fn(),
  listRecentProjects: vi.fn(),
  loadProjectFile: vi.fn(),
  saveProjectFile: vi.fn(),
}));

function materialFixture(): Material {
  return {
    id: "m-1",
    name: "PP-REF-01",
    family: "PP",
    manufacturer: "示例石化",
    rheology: {} as never,
    pvt: {} as never,
    specificHeat: [[200, 2000]] as [number, number][],
    conductivity: [[200, 0.2]] as [number, number][],
    mechanics: null,
    filler: null,
    dataNote: "",
  };
}

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
          injectionTimeS: 1.5,
          vpSwitchVolumePercent: 96,
          packingPressureMpaCurve: [
            [0, 60],
            [8, 48],
          ],
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
    caseDir: "/case",
    cores: 4,
    status: "running",
    createdMs: 1,
    startedMs: 1,
    finishedMs: null,
    lastTimeS: 0.84,
    message: null,
    ...overrides,
  };
}

describe("StudySummaryPanel（方案摘要卡）", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("空状态：三段全部占位指引", () => {
    const wrapper = mount(StudySummaryPanel, { global: { plugins: [pinia] } });
    const text = wrapper.text();
    expect(text).toContain("方案摘要");
    expect(text).toContain("未登记材料");
    expect(text).toContain("未设置工艺");
    expect(text).toContain("尚未提交作业");
  });

  it("材料 id 登记但库中查不到 → 回退空态", () => {
    const project = useProjectStore();
    project.project = projectFixture();
    project.activeStudyId = "s-1";
    // 材料库为空：materialId 悬空
    const wrapper = mount(StudySummaryPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("未登记材料");
  });

  it("材料 / 工艺就绪：prop 行展示牌号与关键参数", () => {
    const project = useProjectStore();
    const materials = useMaterialsStore();
    materials.materials = { builtin: [materialFixture()], custom: [] };
    project.project = projectFixture();
    project.activeStudyId = "s-1";

    const wrapper = mount(StudySummaryPanel, { global: { plugins: [pinia] } });
    const text = wrapper.text();
    expect(text).toContain("PP-REF-01");
    expect(text).toContain("示例石化");
    expect(text).toContain("230 °C");
    expect(text).toContain("40 °C");
    expect(text).toContain("1.5 s");
    expect(text).toContain("96 %");
    // 保压峰值取曲线首点
    expect(text).toContain("60 MPa");
    expect(text).toContain("15 s");
  });

  it("保压曲线为空 → 峰值回退 0", () => {
    const project = useProjectStore();
    const materials = useMaterialsStore();
    materials.materials = { builtin: [materialFixture()], custom: [] };
    const base = projectFixture();
    base.studies[0]!.process = { ...base.studies[0]!.process!, packingPressureMpaCurve: [] };
    project.project = base;
    project.activeStudyId = "s-1";

    const wrapper = mount(StudySummaryPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("0 MPa");
  });

  it("最新作业：状态中文文案 + 运行中的实时物理时间", async () => {
    const jobsStore = useJobsStore();
    const wrapper = mount(StudySummaryPanel, { global: { plugins: [pinia] } });

    jobsStore.jobs = [jobFixture({ status: "running", lastTimeS: 0.84 })];
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("运行中");
    expect(wrapper.text()).toContain("0.84 s");
    expect(wrapper.text()).toContain("4");

    jobsStore.jobs = [jobFixture({ status: "failed", lastTimeS: null })];
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("失败");
    expect(wrapper.text()).not.toContain("0.84 s");
  });
});
