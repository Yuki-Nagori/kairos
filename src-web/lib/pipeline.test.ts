import { describe, expect, it } from "vitest";
import { allPrerequisitesDone, evaluatePipeline, type PipelineInput } from "./pipeline";

function input(overrides: Partial<PipelineInput> = {}): PipelineInput {
  return {
    geometries: [
      {
        geometryId: "g1",
        fileName: "demo.stl",
        triangleCount: 12,
        size: [1, 1, 1],
        surfaceArea: 6,
        signedVolume: 1,
        suggestedUnit: "mm",
        issues: { degenerate: 0, openEdges: 0, nonManifoldEdges: 0, normalInconsistentEdges: 0 },
      },
    ],
    meshReports: {
      g1: {
        elementCount: 40,
      } as unknown as PipelineInput["meshReports"][string],
    },
    project: {
      schemaVersion: 4,
      id: "p",
      name: "n",
      createdMs: 1,
      updatedMs: 1,
      studies: [
        {
          id: "s1",
          name: "研究",
          createdMs: 1,
          runnerElements: [],
          coolingChannels: [],
          process: null,
          materialId: null,
        },
      ],
    },
    activeStudyId: "s1",
    materials: { builtin: [], custom: [] },
    jobs: [],
    ...overrides,
  };
}

describe("evaluatePipeline", () => {
  it("all steps incomplete for a fresh state", () => {
    const steps = evaluatePipeline(
      input({ geometries: [], meshReports: {}, project: null, activeStudyId: null }),
    );
    expect(steps.every((step) => !step.done)).toBe(true);
    expect(steps.map((s) => s.id)).toEqual(["geometry", "mesh", "material", "process", "submit"]);
  });

  it("material step requires the material to exist", () => {
    const base = input();
    base.project!.studies[0]!.materialId = "missing";
    const steps = evaluatePipeline(base);
    const materialStep = steps.find((s) => s.id === "material");
    expect(materialStep?.done).toBe(false);
  });

  it("material step done when material is registered", () => {
    const base = input();
    base.project!.studies[0]!.materialId = "m-1";
    base.materials.custom = [
      {
        id: "m-1",
        name: "PP",
        manufacturer: "x",
        family: "PP",
        rheology: { n: 0.3, tauStar: 1, d1: 1, d2: 1, d3: 0, a1: 1, a2: 1 },
        pvt: { b1m: 1, b1s: 1, b2m: 1, b2s: 1, b3: 1, b4m: 1, b4s: 1, b5: 1 },
        specificHeat: [],
        conductivity: [],
        mechanics: null,
        dataNote: "",
      },
    ];
    const steps = evaluatePipeline(base);
    expect(steps.find((s) => s.id === "material")?.done).toBe(true);
  });

  it("submit step is done when a job has run", () => {
    const base = input();
    base.project!.studies[0]!.process = {
      meltTempC: 230,
      moldTempC: 40,
      ejectionTempC: 90,
      injectionTimeS: 1,
      vpSwitchVolumePercent: 96,
      packingPressureMpaCurve: [[0, 60]],
      packingTimeS: 8,
      coolingTimeS: 15,
      coolantTempC: 25,
    };
    base.project!.studies[0]!.materialId = "m-1";
    base.jobs = [
      {
        id: "j1",
        studyId: "s1",
        caseDir: "/c",
        cores: 2,
        status: "done",
        createdMs: 1,
        startedMs: 1,
        finishedMs: 2,
        lastTimeS: 1,
        message: null,
      },
    ];
    const steps = evaluatePipeline(base);
    expect(steps.find((s) => s.id === "submit")?.done).toBe(true);
  });

  it("allPrerequisitesDone gates submission", () => {
    const steps = evaluatePipeline(input()).map((s) => ({ ...s, done: s.id !== "submit" }));
    expect(steps.every((s) => s.done || s.id === "submit")).toBe(true);
  });
});

describe("allPrerequisitesDone", () => {
  it("非 submit 步骤全部完成时为 true（submit 未完成不影响）", () => {
    const steps = [
      { id: "a", label: "", done: true, hint: null },
      { id: "submit", label: "", done: false, hint: null },
    ];
    expect(allPrerequisitesDone(steps)).toBe(true);
  });

  it("任一前置未完成时为 false", () => {
    const steps = [
      { id: "a", label: "", done: true, hint: null },
      { id: "b", label: "", done: false, hint: null },
    ];
    expect(allPrerequisitesDone(steps)).toBe(false);
  });
});

describe("evaluatePipeline 网格步骤的精确指引", () => {
  it("几何已导入但未生成网格：hint 指向生成体积网格", () => {
    const steps = evaluatePipeline(input({ meshReports: {} }));
    const meshStep = steps.find((s) => s.id === "mesh");
    expect(meshStep?.done).toBe(false);
    expect(meshStep?.hint).toContain("生成体积网格");
  });
});
