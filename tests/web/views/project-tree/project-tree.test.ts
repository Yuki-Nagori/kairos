import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import ProjectTree from "../../../../src-web/views/project-tree/ProjectTree.vue";
import { useProjectTree } from "../../../../src-web/views/project-tree/useProjectTree";
import { useAppStore } from "../../../../src-web/stores/app";
import { useDependenciesStore } from "../../../../src-web/stores/dependencies";
import { useGeometryStore } from "../../../../src-web/stores/geometry";
import { useJobsStore } from "../../../../src-web/stores/jobs";
import { useProjectStore } from "../../../../src-web/stores/project";
import type {
  DependencyStatus,
  GeometrySummary,
  Job,
  Project,
  Study,
} from "../../../../src-web/types";

function geometryFixture(overrides: Partial<GeometrySummary> = {}): GeometrySummary {
  return {
    geometryId: "geo-1",
    fileName: "demo.stl",
    triangleCount: 12,
    size: [10, 20, 30],
    surfaceArea: 2200,
    signedVolume: 6000,
    suggestedUnit: "mm",
    issues: { degenerate: 0, openEdges: 0, nonManifoldEdges: 0, normalInconsistentEdges: 0 },
    ...overrides,
  };
}

function jobFixture(overrides: Partial<Job> = {}): Job {
  return {
    id: "job-1",
    studyId: "study-1",
    caseDir: "/case/job-1",
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

function dependencyFixture(overrides: Partial<DependencyStatus> = {}): DependencyStatus {
  return {
    id: "gmsh",
    name: "Gmsh",
    license: "GPL",
    licenseKind: "gpl",
    strategy: "direct_download",
    pageUrl: "https://gmsh.org",
    required: true,
    checkCommand: "gmsh --version",
    hint: "",
    download: null,
    ready: false,
    managedReady: false,
    updatable: false,
    ...overrides,
  };
}

function projectFixture(overrides: Partial<Project> = {}): Project {
  return {
    schemaVersion: 1,
    id: "p-1",
    name: "演示项目",
    createdMs: 1,
    updatedMs: 1,
    studies: [],
    ...overrides,
  };
}

function studyFixture(overrides: Partial<Study> = {}): Study {
  return {
    id: "study-1",
    name: "方案 A",
    createdMs: 1,
    runnerElements: [],
    coolingChannels: [],
    process: null,
    materialId: null,
    ...overrides,
  };
}

describe("ProjectTree", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("未打开项目时只渲染占位叶，空分组被过滤", () => {
    const wrapper = mount(ProjectTree, { global: { plugins: [pinia] } });
    const text = wrapper.text();
    expect(text).toContain("工程");
    // 头部徽标：未打开项目时给占位名
    expect(text).toContain("未打开项目");
    // 方案层与新建入口都要求先有工程。
    expect(text).not.toContain("方案");
    expect(wrapper.find('button[title="新建方案"]').exists()).toBe(false);
    // 空分组不渲染标题。
    expect(text).not.toContain("几何");
    expect(text).not.toContain("求解作业");
    expect(text).not.toContain("运行时依赖");
  });

  it("按 store 状态渲染几何 / 作业 / 依赖分组", () => {
    const geometry = useGeometryStore();
    const jobsStore = useJobsStore();
    const deps = useDependenciesStore();
    const project = useProjectStore();

    geometry.geometries = [geometryFixture()];
    jobsStore.jobs = [
      jobFixture({ id: "job-1", status: "running" }),
      jobFixture({ id: "job-2", status: "done" }),
    ];
    deps.dependencies = [
      dependencyFixture({ name: "Gmsh", ready: false }),
      dependencyFixture({ id: "python", name: "Python", ready: true }),
    ];
    project.project = projectFixture();

    const wrapper = mount(ProjectTree, { global: { plugins: [pinia] } });
    const text = wrapper.text();
    expect(text).toContain("演示项目");
    expect(text).toContain("demo.stl (12 面)");
    expect(text).toContain("job-1: running");
    expect(text).toContain("job-2: done");
    expect(text).toContain("Gmsh: 未就绪");
    expect(text).toContain("Python: 就绪");
  });

  it("方案层渲染并可点击切换活跃研究", async () => {
    const project = useProjectStore();
    project.project = projectFixture({
      studies: [studyFixture(), studyFixture({ id: "study-2", name: "方案 B", createdMs: 2 })],
    });
    project.activeStudyId = "study-1";

    const wrapper = mount(ProjectTree, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("方案");
    const buttons = wrapper.findAll('button[title^="切换到方案"]');
    expect(buttons).toHaveLength(2);
    // 活跃方案高亮
    expect(buttons[0]!.classes()).toContain("bg-emerald-900/40");
    expect(buttons[1]!.classes()).not.toContain("bg-emerald-900/40");

    await buttons[1]!.trigger("click");
    expect(project.activeStudyId).toBe("study-2");
    await wrapper.vm.$nextTick();
    expect(buttons[1]!.classes()).toContain("bg-emerald-900/40");
  });

  it("＋ 新建方案：按序号补名并立即成为活跃方案", async () => {
    const project = useProjectStore();
    project.project = projectFixture({ studies: [studyFixture({ id: "study-1" })] });
    project.activeStudyId = "study-1";

    const wrapper = mount(ProjectTree, { global: { plugins: [pinia] } });
    await wrapper.find('button[title="新建方案"]').trigger("click");

    expect(project.project?.studies.map((study) => study.name)).toEqual(["方案 A", "方案 2"]);
    expect(project.activeStudyId).toBe(project.project?.studies[1]?.id);
    // 新方案行渲染为活跃态。
    const buttons = wrapper.findAll('button[title^="切换到方案"]');
    expect(buttons).toHaveLength(2);
    expect(buttons[1]!.classes()).toContain("bg-emerald-900/40");
  });

  it("无方案的旧工程给出空态提示，入口仍可用", async () => {
    const project = useProjectStore();
    project.project = projectFixture({ studies: [] });

    const wrapper = mount(ProjectTree, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("尚无方案——点「＋」新建。");

    await wrapper.find('button[title="新建方案"]').trigger("click");
    expect(project.project?.studies.map((study) => study.name)).toEqual(["方案 1"]);
  });

  it("未打开工程时方案层为空列表，新建动作被 store 拒绝", () => {
    const tree = useProjectTree();

    expect(tree.hasProject.value).toBe(false);
    expect(tree.studies.value).toEqual([]);

    tree.createStudy();

    expect(useAppStore().error?.message).toBe("请先新建或打开项目。");
  });
});
