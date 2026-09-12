import { describe, expect, it } from "vitest";
import {
  evaluateStudyTasks,
  prerequisitesReady,
  type StudyTasksInput,
} from "../../../src-web/utils/study-tasks";
import type { GeometrySummary, Job, Material, Project } from "../../../src-web/types";

function geometry(overrides: Partial<GeometrySummary> = {}): GeometrySummary {
  return {
    geometryId: "g-1",
    fileName: "mug.stl",
    triangleCount: 1200,
    size: [20, 20, 30],
    surfaceArea: 3000,
    signedVolume: 8000,
    suggestedUnit: "mm",
    issues: { degenerate: 0, openEdges: 0, nonManifoldEdges: 0, normalInconsistentEdges: 0 },
    ...overrides,
  };
}

function material(id: string, name: string): Material {
  return {
    id,
    name,
    family: "PP",
    manufacturer: "示例",
    rheology: {} as Material["rheology"],
    pvt: {} as Material["pvt"],
    specificHeat: [[200, 2000]],
    thermalConductivity: [[200, 0.2]],
    mechanics: null,
    filler: null,
    dataNote: "",
  } as unknown as Material;
}

function project(overrides: { materialId?: string | null; withProcess?: boolean }): Project {
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
        process: overrides.withProcess
          ? {
              meltTempC: 230,
              moldTempC: 40,
              ejectionTempC: 90,
              injectionTimeS: 1,
              vpSwitchVolumePercent: 96,
              packingPressureMpaCurve: [[0, 60]],
              packingTimeS: 8,
              coolingTimeS: 15,
              coolantTempC: 25,
            }
          : null,
        materialId: overrides.materialId ?? null,
      },
    ],
  };
}

function job(overrides: Partial<Job>): Job {
  return {
    id: "job-1",
    studyId: "s-1",
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

function input(overrides: Partial<StudyTasksInput> = {}): StudyTasksInput {
  return {
    geometries: [],
    meshReports: {},
    project: null,
    activeStudyId: "s-1",
    materials: { builtin: [], custom: [] },
    jobs: [],
    resultCatalog: null,
    ...overrides,
  };
}

function byId(tasks: ReturnType<typeof evaluateStudyTasks>, id: string) {
  const task = tasks.find((task) => task.id === id);
  if (task === undefined) {
    throw new Error(`任务 ${id} 不存在`);
  }
  return task;
}

describe("evaluateStudyTasks（方案任务序列）", () => {
  it("空状态：全部待办，任务按 Moldflow 执行顺序排列", () => {
    const tasks = evaluateStudyTasks(input());
    expect(tasks.map((task) => task.id)).toEqual([
      "geometry",
      "material",
      "process",
      "analysis",
      "results",
    ]);
    expect(tasks.every((task) => task.state === "todo")).toBe(true);
  });

  it("几何就绪但未网格化：导入完成，网格待办并带指引", () => {
    const tasks = evaluateStudyTasks(input({ geometries: [geometry()] }));
    expect(byId(tasks, "geometry").state).toBe("done");
    expect(byId(tasks, "geometry").detail).toBe("mug.stl（1200 面）");
    expect(byId(tasks, "mesh").state).toBe("todo");
    expect(byId(tasks, "mesh").hint).toContain("生成体积网格");
  });

  it("网格完成且几何健康：网格 done；不健康 → warning 并追加修复任务", () => {
    const healthy = evaluateStudyTasks(
      input({ geometries: [geometry()], meshReports: { "g-1": { elementCount: 5000 } } }),
    );
    expect(byId(healthy, "mesh").state).toBe("done");
    expect(byId(healthy, "mesh").detail).toBe("四面体 5000");
    expect(healthy.some((task) => task.id === "repair")).toBe(false);

    const unhealthy = evaluateStudyTasks(
      input({
        geometries: [
          geometry({
            issues: {
              degenerate: 2,
              openEdges: 4,
              nonManifoldEdges: 0,
              normalInconsistentEdges: 0,
            },
          }),
        ],
        meshReports: { "g-1": { elementCount: 5000 } },
      }),
    );
    expect(byId(unhealthy, "mesh").state).toBe("warning");
    expect(byId(unhealthy, "repair").state).toBe("todo");
    // 修复任务在网格之后、材料之前（诊断/修复紧跟划分）
    expect(unhealthy.findIndex((task) => task.id === "repair")).toBe(2);
  });

  it("材料与工艺：研究登记材料且工艺应用后完成，详情带摘要", () => {
    const tasks = evaluateStudyTasks(
      input({
        geometries: [geometry()],
        meshReports: { "g-1": { elementCount: 5000 } },
        project: project({ materialId: "m-1", withProcess: true }),
        materials: { builtin: [material("m-1", "PP-REF-01")], custom: [] },
      }),
    );
    expect(byId(tasks, "material").state).toBe("done");
    expect(byId(tasks, "material").detail).toBe("PP-REF-01");
    expect(byId(tasks, "process").state).toBe("done");
    expect(byId(tasks, "process").detail).toContain("230");
  });

  it("分析任务状态映射：排队 ⧖ / 运行 ⟳ / 完成 ✓ / 失败 ✕", () => {
    const base = {
      geometries: [geometry()],
      project: project({ materialId: "m-1", withProcess: true }),
    };
    const mk = (jobs: Job[]) => evaluateStudyTasks(input({ ...base, jobs }));

    expect(byId(mk([job({ status: "queued" })]), "analysis").state).toBe("queued");
    expect(byId(mk([job({ status: "running", lastTimeS: 0.4 })]), "analysis").state).toBe(
      "running",
    );
    expect(byId(mk([job({ status: "done", finishedMs: 2 })]), "analysis").state).toBe("done");
    expect(byId(mk([job({ status: "failed", message: "boom" })]), "analysis").state).toBe("failed");
    // 混合队列：执行中优先于排队（消融 T1 锁定——单一状态夹具区分不了优先级）
    expect(
      byId(
        mk([
          job({ id: "job-q", status: "queued" }),
          job({ id: "job-r", status: "running", lastTimeS: 0.3 }),
        ]),
        "analysis",
      ).state,
    ).toBe("running");
    // 终态混合：以最后一个作业为准（失败 > 完成）
    expect(
      byId(
        mk([
          job({ id: "job-d", status: "done", finishedMs: 2 }),
          job({ id: "job-f", status: "failed", message: "boom" }),
        ]),
        "analysis",
      ).state,
    ).toBe("failed");
    // 全部已取消 → 回到未开始（可重新提交）
    expect(byId(mk([job({ status: "cancelled", finishedMs: 3 })]), "analysis").state).toBe("todo");
  });

  it("失败阻断：分析失败 → 结果任务 blocked 并带原因（Moldflow 规则）", () => {
    const tasks = evaluateStudyTasks(
      input({
        jobs: [job({ status: "failed" })],
        resultCatalog: null,
      }),
    );
    const results = byId(tasks, "results");
    expect(results.state).toBe("blocked");
    expect(results.blockReason).toContain("分析作业失败");
    // 失败阻断时不再展示常规指引（hint 置空分支）
    expect(results.hint).toBeNull();
  });

  it("运行 / 排队中：结果任务 blocked（等待分析完成）而非待办", () => {
    const tasks = evaluateStudyTasks(input({ jobs: [job({ status: "running", lastTimeS: 0.8 })] }));
    expect(byId(tasks, "results").state).toBe("blocked");
    expect(byId(tasks, "results").blockReason).toBe("等待分析完成。");
  });

  it("结果目录扫描后：结果任务 done 并带时间步数", () => {
    const tasks = evaluateStudyTasks(
      input({
        resultCatalog: {
          caseDir: "/case",
          times: [
            { dirName: "1", timeS: 1, fields: ["p"] },
            { dirName: "2", timeS: 2, fields: ["p"] },
          ],
        },
      }),
    );
    expect(byId(tasks, "results").state).toBe("done");
    expect(byId(tasks, "results").detail).toBe("2 个时间步");
  });

  it("prerequisitesReady：结果任务不参与前置判断；warning 不阻断提交", () => {
    const unhealthy = evaluateStudyTasks(
      input({
        geometries: [
          geometry({
            issues: {
              degenerate: 1,
              openEdges: 2,
              nonManifoldEdges: 0,
              normalInconsistentEdges: 0,
            },
          }),
        ],
        meshReports: { "g-1": { elementCount: 5000 } },
        project: project({ materialId: "m-1", withProcess: true }),
        materials: { builtin: [material("m-1", "PP")], custom: [] },
      }),
    );
    // 网格 warning + 修复任务 todo —— 修复任务未完成则不可提交
    expect(prerequisitesReady(unhealthy)).toBe(false);
  });
});
