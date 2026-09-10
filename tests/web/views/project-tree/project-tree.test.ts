import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import ProjectTree from "../../../../src-web/views/project-tree/ProjectTree.vue";
import { useDependenciesStore } from "../../../../src-web/stores/dependencies";
import { useGeometryStore } from "../../../../src-web/stores/geometry";
import { useJobsStore } from "../../../../src-web/stores/jobs";
import { useProjectStore } from "../../../../src-web/stores/project";
import type { DependencyStatus, GeometrySummary, Job, Project } from "../../../../src-web/types";

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
});
