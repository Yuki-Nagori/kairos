import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAppStore } from "../../../src-web/stores/app";
import { useGeometryStore } from "../../../src-web/stores/geometry";
import { useJobsStore } from "../../../src-web/stores/jobs";
import { useMaterialsStore } from "../../../src-web/stores/materials";
import { usePipelineStore } from "../../../src-web/stores/pipeline";
import { useProjectStore } from "../../../src-web/stores/project";
import { defaultCaseDir } from "../../../src-web/api/project";
import { generateMoldingfoamCase } from "../../../src-web/api/solver";
import { listJobs, submitJob } from "../../../src-web/api/jobs";
import type {
  GeometrySummary,
  Job,
  Material,
  MeshingReport,
  ProcessSettings,
  Study,
} from "../../../src-web/types";

// happy-dom 下没有 Tauri IPC：用可赋值 onmessage 的桩替换 Channel。
vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: ((message: unknown) => void) | null = null;
  },
}));
vi.mock("../../../src-web/api/project", () => ({
  createProject: vi.fn(),
  listRecentProjects: vi.fn(),
  loadProjectFile: vi.fn(),
  saveProjectFile: vi.fn(),
  defaultCaseDir: vi.fn(),
}));
vi.mock("../../../src-web/api/solver", () => ({
  generateMoldingfoamCase: vi.fn(),
  probeMoldingfoam: vi.fn(),
}));
vi.mock("../../../src-web/api/jobs", () => ({
  submitJob: vi.fn(),
  cancelJob: vi.fn(),
  listJobs: vi.fn(),
}));

const geometry: GeometrySummary = {
  geometryId: "g-1",
  fileName: "box.stl",
  triangleCount: 12,
  size: [10, 10, 10],
  surfaceArea: 600,
  signedVolume: 1000,
  suggestedUnit: "mm",
  issues: { degenerate: 0, openEdges: 0, nonManifoldEdges: 0, normalInconsistentEdges: 0 },
};

const meshReport: MeshingReport = {
  engine: "voxel",
  nodeCount: 8,
  elementCount: 6,
  surfaceFaceCount: 12,
  totalVolume: 1000,
  quality: { minEdgeRatio: 1, avgEdgeRatio: 1.2, maxEdgeRatio: 2, minVolume: 0.5 },
  thinFeatureHints: [],
};

const process: ProcessSettings = {
  meltTempC: 230,
  moldTempC: 40,
  ejectionTempC: 90,
  injectionTimeS: 1.2,
  vpSwitchVolumePercent: 95,
  packingPressureMpaCurve: [[0, 60]],
  packingTimeS: 6,
  coolingTimeS: 15,
  coolantTempC: 25,
};

const material: Material = {
  id: "builtin-pp",
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
  mechanics: { elasticModulus: 1.5e9, poissonRatio: 0.35 },
  filler: null,
  dataNote: "示例数据",
};

function makeStudy(overrides: Partial<Study> = {}): Study {
  return {
    id: "s-1",
    name: "填充分析",
    createdMs: 1,
    runnerElements: [],
    coolingChannels: [],
    process,
    materialId: "builtin-pp",
    ...overrides,
  };
}

function makeJob(): Job {
  return {
    id: "job-1",
    studyId: "s-1",
    caseDir: "/data/cases/s-1",
    cores: 4,
    status: "queued",
    createdMs: 1,
    startedMs: null,
    finishedMs: null,
    lastTimeS: null,
    message: null,
  };
}

/** 前置条件的防御校验按「几何 → 网格 → 材料 → 工艺」顺序短路。 */
function primeStores(options: { withGeometry?: boolean; withMesh?: boolean; study?: Study }): void {
  const geometryStore = useGeometryStore();
  geometryStore.geometries = options.withGeometry === false ? [] : [geometry];
  geometryStore.meshReports = options.withMesh === false ? {} : { "g-1": meshReport };

  const materialsStore = useMaterialsStore();
  materialsStore.materials = { builtin: [material], custom: [] };

  const study = options.study ?? makeStudy();
  const projectStore = useProjectStore();
  projectStore.project = {
    schemaVersion: 1,
    id: "p-1",
    name: "项目",
    createdMs: 1,
    updatedMs: 1,
    studies: [study],
  };
  projectStore.activeStudyId = study.id;
}

describe("pipeline store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetAllMocks();
    vi.mocked(listJobs).mockResolvedValue([]);
    vi.mocked(submitJob).mockResolvedValue(makeJob());
  });

  it("refuses to run without an imported geometry", async () => {
    primeStores({ withGeometry: false });

    const app = useAppStore();
    const pipeline = usePipelineStore();
    await pipeline.submitPipeline(4, "fill");

    expect(app.error?.message).toBe("请先导入几何。");
    expect(generateMoldingfoamCase).not.toHaveBeenCalled();
    expect(app.busy).toBeNull();
  });

  it("refuses to run without a volume mesh", async () => {
    primeStores({ withMesh: false });

    const app = useAppStore();
    const pipeline = usePipelineStore();
    await pipeline.submitPipeline(4, "fill");

    expect(app.error?.message).toBe("请先生成体积网格。");
    expect(generateMoldingfoamCase).not.toHaveBeenCalled();
  });

  it("refuses to run without a registered material", async () => {
    primeStores({ study: makeStudy({ materialId: null }) });

    const app = useAppStore();
    const pipeline = usePipelineStore();
    await pipeline.submitPipeline(4, "fill");

    expect(app.error?.message).toBe("请先在研究上登记材料。");
    expect(generateMoldingfoamCase).not.toHaveBeenCalled();
  });

  it("refuses to run when the registered material id is unknown", async () => {
    primeStores({ study: makeStudy({ materialId: "ghost-material" }) });

    const app = useAppStore();
    const pipeline = usePipelineStore();
    await pipeline.submitPipeline(4, "fill");

    expect(app.error?.message).toBe("请先在研究上登记材料。");
  });

  it("refuses to run without process settings on the study", async () => {
    primeStores({ study: makeStudy({ process: null }) });

    const app = useAppStore();
    const pipeline = usePipelineStore();
    await pipeline.submitPipeline(4, "fill");

    expect(app.error?.message).toBe("请先设置工艺并应用到研究。");
    expect(generateMoldingfoamCase).not.toHaveBeenCalled();
  });

  it("generates the case and submits the job end to end", async () => {
    primeStores({});
    vi.mocked(defaultCaseDir).mockResolvedValue("/data/cases/s-1");
    vi.mocked(listJobs).mockResolvedValue([makeJob()]);

    const app = useAppStore();
    const pipeline = usePipelineStore();
    const busyDuring: (string | null)[] = [];
    vi.mocked(generateMoldingfoamCase).mockImplementation(async () => {
      busyDuring.push(useAppStore().busy);
      return "/data/cases/s-1";
    });

    await pipeline.submitPipeline(4, "fill_pack");

    expect(defaultCaseDir).toHaveBeenCalledWith("s-1");
    expect(generateMoldingfoamCase).toHaveBeenCalledWith({
      geometryId: "g-1",
      caseDir: "/data/cases/s-1",
      material,
      process,
      stage: "fill_pack",
      cores: 4,
      runnerElements: expect.any(Array),
    });
    // 作业提交走 jobs store：同一 case 目录 + 核数 + 活跃研究 id。
    expect(submitJob).toHaveBeenCalledWith("/data/cases/s-1", 4, "s-1", expect.anything());
    expect(useJobsStore().jobs).toHaveLength(1);
    expect(busyDuring).toEqual(["正在准备求解…"]);
    expect(app.busy).toBeNull();
    expect(app.error).toBeNull();
  });

  it("reports case generation failures and clears busy", async () => {
    primeStores({});
    vi.mocked(defaultCaseDir).mockResolvedValue("/data/cases/s-1");
    vi.mocked(generateMoldingfoamCase).mockRejectedValue(new Error("case 生成失败"));

    const app = useAppStore();
    const pipeline = usePipelineStore();
    await pipeline.submitPipeline(4, "fill");

    expect(app.error?.message).toBe("case 生成失败");
    expect(app.busy).toBeNull();
    expect(submitJob).not.toHaveBeenCalled();
  });

  it("reports defaultCaseDir failures", async () => {
    primeStores({});
    vi.mocked(defaultCaseDir).mockRejectedValue(new Error("数据目录不可用"));

    const app = useAppStore();
    const pipeline = usePipelineStore();
    await pipeline.submitPipeline(4, "fill");

    expect(app.error?.message).toBe("数据目录不可用");
    expect(app.busy).toBeNull();
    expect(generateMoldingfoamCase).not.toHaveBeenCalled();
  });
});
