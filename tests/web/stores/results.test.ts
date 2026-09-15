import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAppStore } from "../../../src-web/stores/app";
import { useProjectStore } from "../../../src-web/stores/project";
import { downloadTextFile } from "../../../src-web/utils/download";
import { useResultsStore } from "../../../src-web/stores/results";
import {
  deformRenderMesh,
  loadTensorField,
  sampleProbeSeries,
  deriveDifference as deriveDifferenceApi,
  deriveField as deriveFieldApi,
  listResultTimes,
  loadResultField,
  loadVectorField,
} from "../../../src-web/api/results";
import { analyzeGateLocation, previewFill } from "../../../src-web/api/geometry";
import type {
  FillPreviewReport,
  GateLocationReport,
  ResultCatalog,
  ScalarField,
} from "../../../src-web/types";

vi.mock("../../../src-web/api/results", () => ({
  listResultTimes: vi.fn(),
  loadResultField: vi.fn(),
  loadVectorField: vi.fn(),
  loadTensorField: vi.fn(),
  sampleProbeSeries: vi.fn(),
  deformRenderMesh: vi.fn(),
  deriveField: vi.fn(),
  deriveDifference: vi.fn(),
}));
vi.mock("../../../src-web/api/geometry", () => ({
  analyzeGateLocation: vi.fn(),
  previewFill: vi.fn(),
}));
vi.mock("../../../src-web/utils/download", () => ({ downloadTextFile: vi.fn() }));

const catalog: ResultCatalog = {
  caseDir: "/case/run",
  times: [{ dirName: "0.100", timeS: 0.1, fields: ["p", "U"] }],
};

function makeField(overrides: Partial<ScalarField> = {}): ScalarField {
  return {
    field: "p",
    timeDir: "0.100",
    timeS: 0.1,
    values: [1, 2, 3],
    isMagnitude: true,
    complete: true,
    ...overrides,
  };
}

describe("results store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetAllMocks();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  describe("loadResultsCatalog", () => {
    it("stores the scanned catalog and clears busy", async () => {
      vi.mocked(listResultTimes).mockResolvedValue(catalog);

      const app = useAppStore();
      const results = useResultsStore();
      const busyDuring: (string | null)[] = [];
      vi.mocked(listResultTimes).mockImplementation(async () => {
        busyDuring.push(useAppStore().busy);
        return catalog;
      });
      await results.loadResultsCatalog("/case/run");

      expect(listResultTimes).toHaveBeenCalledWith("/case/run");
      expect(results.resultCatalog).toEqual(catalog);
      expect(busyDuring).toEqual(["正在扫描结果…"]);
      expect(app.busy).toBeNull();
      expect(app.error).toBeNull();
    });

    it("reports scan failures and clears busy", async () => {
      vi.mocked(listResultTimes).mockRejectedValue(new Error("case 目录不存在"));

      const app = useAppStore();
      const results = useResultsStore();
      await results.loadResultsCatalog("/case/missing");

      expect(app.error?.message).toBe("case 目录不存在");
      expect(results.resultCatalog).toBeNull();
      expect(app.busy).toBeNull();
    });
  });

  describe("rescanCatalog", () => {
    it("没有目录时引导先去结果面板扫描", async () => {
      const app = useAppStore();
      const results = useResultsStore();
      await results.rescanCatalog();

      expect(listResultTimes).not.toHaveBeenCalled();
      expect(app.error?.message).toBe("请先在结果面板填写 case 目录并扫描。");
    });

    it("用上次扫描的目录重扫", async () => {
      vi.mocked(listResultTimes).mockResolvedValue(catalog);
      const results = useResultsStore();
      await results.loadResultsCatalog("/case/run");
      vi.mocked(listResultTimes).mockClear();

      await results.rescanCatalog();
      expect(listResultTimes).toHaveBeenCalledWith("/case/run");
    });
  });

  describe("loadField", () => {
    it("stores the loaded field and clears busy", async () => {
      const field = makeField();
      vi.mocked(loadResultField).mockResolvedValue(field);

      const app = useAppStore();
      const results = useResultsStore();
      await results.loadField("/case/run", "0.100", "p");

      expect(loadResultField).toHaveBeenCalledWith("/case/run", "0.100", "p", "primary");
      expect(results.loadedField).toEqual(field);
      expect(app.busy).toBeNull();
      expect(app.error).toBeNull();
    });

    it("reports load failures and clears busy", async () => {
      vi.mocked(loadResultField).mockRejectedValue(new Error("场文件缺失"));

      const app = useAppStore();
      const results = useResultsStore();
      await results.loadField("/case/run", "0.100", "p");

      expect(app.error?.message).toBe("场文件缺失");
      expect(results.loadedField).toBeNull();
      expect(app.busy).toBeNull();
    });
  });

  describe("addProbe", () => {
    it("rejects non-integer and negative node indices", () => {
      const app = useAppStore();
      const results = useResultsStore();

      results.addProbe(1.5);
      expect(app.error?.message).toContain("非负整数");
      expect(results.probes).toEqual([]);

      results.addProbe(-1);
      expect(app.error?.message).toContain("非负整数");
      expect(results.probes).toEqual([]);
    });

    it("rejects duplicate probes", () => {
      const app = useAppStore();
      const results = useResultsStore();
      results.addProbe(0);

      results.addProbe(0);
      expect(app.error?.message).toContain("节点 0 已有探针");
      expect(results.probes).toHaveLength(1);
    });

    it("rejects indices beyond the loaded field", () => {
      const app = useAppStore();
      const results = useResultsStore();
      results.loadedField = makeField({ values: [1, 2, 3] });

      results.addProbe(3);
      expect(app.error?.message).toContain("节点序号超出范围");
      expect(app.error?.message).toContain("3 个值");
      expect(results.probes).toEqual([]);
    });

    it("appends valid probes with incrementing ids", () => {
      const app = useAppStore();
      const results = useResultsStore();
      // 未加载场时不做范围校验。
      results.addProbe(0);
      results.loadedField = makeField();
      results.addProbe(2);

      expect(app.error).toBeNull();
      expect(results.probes.map((probe) => probe.nodeIndex)).toEqual([0, 2]);
      // id 严格递增（模块级序号跨测试共享，只断言相对关系）。
      const [first, second] = results.probes;
      expect(second?.id).toBe((first?.id ?? 0) + 1);
    });
  });

  describe("removeProbe", () => {
    it("removes the probe by id", () => {
      const results = useResultsStore();
      results.addProbe(1);
      results.addProbe(2);
      const firstId = results.probes[0]?.id ?? 0;

      results.removeProbe(firstId);

      expect(results.probes.map((probe) => probe.nodeIndex)).toEqual([2]);
    });
  });

  describe("exportFieldCsv", () => {
    it("reports when no field has been loaded", () => {
      const app = useAppStore();
      const results = useResultsStore();

      results.exportFieldCsv();

      expect(app.error?.message).toContain("暂无可导出的场数据");
      expect(downloadTextFile).not.toHaveBeenCalled();
    });

    it("reports when the loaded field has no values", () => {
      const app = useAppStore();
      const results = useResultsStore();
      results.loadedField = makeField({ values: [] });

      results.exportFieldCsv();

      expect(app.error?.message).toContain("暂无可导出的场数据");
    });

    it("downloads a magnitude CSV named after the field and time", () => {
      const app = useAppStore();
      const results = useResultsStore();
      results.loadedField = makeField();
      results.exportFieldCsv();

      // DOM 机制在 utils/download（其自身测试覆盖）；这里只断言数据与文件名。
      expect(downloadTextFile).toHaveBeenCalledTimes(1);
      expect(downloadTextFile).toHaveBeenCalledWith(
        "p-0.100.csv",
        "\uFEFFnode,p (magnitude)\r\n0,1\r\n1,2\r\n2,3\r\n",
      );
      expect(app.error).toBeNull();
    });

    it("keeps the plain header for non-magnitude fields", () => {
      const results = useResultsStore();
      results.loadedField = makeField({ field: "T", isMagnitude: false });
      results.exportFieldCsv();

      expect(downloadTextFile).toHaveBeenCalledWith(
        "T-0.100.csv",
        "\uFEFFnode,T\r\n0,1\r\n1,2\r\n2,3\r\n",
      );
    });
  });

  describe("deriveField", () => {
    it("没有已加载场时静默返回，不触发派生请求", async () => {
      const app = useAppStore();
      const results = useResultsStore();
      await results.deriveField({ kind: "normalize" });
      expect(deriveFieldApi).not.toHaveBeenCalled();
      expect(app.error).toBeNull();
    });

    it("调用后端派生并写回 loadedField", async () => {
      vi.mocked(listResultTimes).mockResolvedValue(catalog);
      const results = useResultsStore();
      await results.loadResultsCatalog("/case/run");

      vi.mocked(loadResultField).mockResolvedValue(makeField());
      await results.loadField("/case/run", "0.100", "p");
      vi.mocked(deriveFieldApi).mockResolvedValue(
        makeField({ field: "p · 阈值掩码", values: [0, 1, 0] }),
      );

      await results.deriveField({ kind: "threshold" });
      expect(deriveFieldApi).toHaveBeenCalledWith({ kind: "threshold" });
      expect(results.loadedField?.values).toEqual([0, 1, 0]);
    });

    it("派生请求失败时错误进入全局状态", async () => {
      const app = useAppStore();
      const results = useResultsStore();
      await results.loadResultsCatalog("/case/run");
      vi.mocked(loadResultField).mockResolvedValue(makeField());
      await results.loadField("/case/run", "0.100", "p");

      vi.mocked(deriveFieldApi).mockRejectedValue(new Error("派生失败"));
      await results.deriveField({ kind: "normalize" });
      expect(app.error?.message).toBe("派生失败");
    });

    it("线性映射请求携带 scale 与 offset", async () => {
      vi.mocked(listResultTimes).mockResolvedValue(catalog);
      const results = useResultsStore();
      await results.loadResultsCatalog("/case/run");
      vi.mocked(loadResultField).mockResolvedValue(makeField());
      await results.loadField("/case/run", "0.100", "p");
      vi.mocked(deriveFieldApi).mockResolvedValue(makeField());

      await results.deriveField({ kind: "linear", scale: 2, offset: -1 });

      expect(deriveFieldApi).toHaveBeenCalledWith({ kind: "linear", scale: 2, offset: -1 });
    });
  });

  describe("loadProbeTimeSeries", () => {
    function makeProbe(nodeIndex: number) {
      return { id: nodeIndex, nodeIndex };
    }

    it("采样只请求曲线，主场保持原时间步且不发送逐场加载", async () => {
      const results = useResultsStore();
      results.resultCatalog = {
        caseDir: "/case/run",
        times: [{ dirName: "1", timeS: 1, fields: ["T"] }],
      };
      results.probes = [makeProbe(0)];
      results.loadedField = makeField({ field: "T", timeDir: "0" });
      const original = results.loadedField;
      const series = [{ probeId: 0, nodeIndex: 0, samples: [{ timeS: 1, value: 42 }] }];
      vi.mocked(sampleProbeSeries).mockResolvedValue(series);
      await results.loadProbeTimeSeries();
      expect(sampleProbeSeries).toHaveBeenCalledWith("/case/run", "T", results.probes);
      expect(loadResultField).not.toHaveBeenCalled();
      expect(results.loadedField).toBe(original);
      expect(results.probeTimeSeries).toEqual(series);
      expect(results.probeSeriesField).toBe("T");
    });

    it("目录 / 探针 / 已加载场缺失时静默返回", async () => {
      const results = useResultsStore();
      await results.loadProbeTimeSeries();
      expect(loadResultField).not.toHaveBeenCalled();
    });

    it("时间步加载失败时错误进入全局状态", async () => {
      vi.mocked(listResultTimes).mockResolvedValue({
        caseDir: "/case/run",
        times: [{ dirName: "0", timeS: 0, fields: ["T"] }],
      });
      vi.mocked(sampleProbeSeries).mockRejectedValue(new Error("时间步缺失"));

      const app = useAppStore();
      const results = useResultsStore();
      await results.loadResultsCatalog("/case/run");
      results.probes = [makeProbe(0)];
      results.loadedField = makeField();
      await results.loadProbeTimeSeries();

      expect(app.error?.message).toBe("时间步缺失");
    });
  });

  describe("compare slot & deriveDifference", () => {
    it("compare 槽位加载写入 compareField 而非 loadedField", async () => {
      const primary = makeField();
      vi.mocked(loadResultField).mockResolvedValue(primary);

      const results = useResultsStore();
      await results.loadField("/case/run", "0.100", "p", "compare");

      expect(loadResultField).toHaveBeenCalledWith("/case/run", "0.100", "p", "compare");
      expect(results.compareField).toEqual(primary);
      expect(results.loadedField).toBeNull();
    });

    it("双场就绪时执行差值并写回 loadedField", async () => {
      const primary = makeField();
      vi.mocked(loadResultField).mockResolvedValue(primary);
      const results = useResultsStore();
      await results.loadField("/case/run", "0.100", "p");
      await results.loadField("/case/run", "0.100", "T", "compare");
      vi.mocked(deriveDifferenceApi).mockResolvedValue(makeField({ field: "p - T" }));

      await results.deriveDifference();

      expect(deriveDifferenceApi).toHaveBeenCalled();
      expect(results.loadedField?.field).toBe("p - T");
    });

    it("移除探针时同步过滤其时间序列", () => {
      const results = useResultsStore();
      results.probes = [
        { id: 1, nodeIndex: 0 },
        { id: 2, nodeIndex: 1 },
      ];
      results.probeTimeSeries = [
        { probeId: 1, nodeIndex: 0, samples: [{ timeS: 0, value: 1 }] },
        { probeId: 2, nodeIndex: 1, samples: [{ timeS: 0, value: 2 }] },
      ];

      results.removeProbe(1);

      expect(results.probes.map((probe) => probe.id)).toEqual([2]);
      expect(results.probeTimeSeries.map((series) => series.probeId)).toEqual([2]);
    });

    it("缺少主场或对比场时静默返回，不触发差值请求", async () => {
      const results = useResultsStore();
      await results.deriveDifference();
      expect(deriveDifferenceApi).not.toHaveBeenCalled();
    });

    it("差值请求失败时错误进入全局状态", async () => {
      vi.mocked(loadResultField).mockResolvedValue(makeField());
      const results = useResultsStore();
      await results.loadField("/case/run", "0.100", "p");
      await results.loadField("/case/run", "0.100", "T", "compare");
      vi.mocked(deriveDifferenceApi).mockRejectedValue(new Error("差值失败"));

      const app = useAppStore();
      await results.deriveDifference();

      expect(app.error?.message).toBe("差值失败");
    });
  });
});

describe("runGateLocation（浇口位置分析）", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetAllMocks();
  });

  const report: GateLocationReport = {
    field: [0.9, 0.5, 0.1],
    candidateCount: 2,
    cellCount: 3,
    top: [
      {
        cell: 0,
        node: 4,
        center: [1, 2, 3],
        score: 0.9,
        maxFlowLengthMm: 12.5,
        thicknessMm: 2,
      },
    ],
    diagonalMm: 17.3,
    basis: "流动长度均衡 × 壁厚可达性（候选限表面单元；启发式建议，非求解结果）",
  };

  it("运行后报告入状态，适合度场直接作为当前场", async () => {
    vi.mocked(analyzeGateLocation).mockResolvedValue(report);
    const app = useAppStore();
    const results = useResultsStore();
    const busyDuring: (string | null)[] = [];
    vi.mocked(analyzeGateLocation).mockImplementation(async () => {
      busyDuring.push(useAppStore().busy);
      return report;
    });

    await results.runGateLocation("g-1", 3);

    expect(analyzeGateLocation).toHaveBeenCalledWith("g-1", 3);
    expect(busyDuring).toEqual(["正在分析浇口位置…"]);
    expect(results.gateLocation).toEqual(report);
    expect(results.loadedField).toEqual({
      field: "浇口适合度",
      timeDir: "—",
      timeS: 0,
      values: report.field,
      isMagnitude: false,
      complete: true,
    });
    expect(app.busy).toBeNull();
    expect(app.error).toBeNull();
  });

  it("默认 Top-N 为 5；失败进全局错误且不覆盖已有报告", async () => {
    vi.mocked(analyzeGateLocation).mockResolvedValue(report);
    const results = useResultsStore();
    await results.runGateLocation("g-1");
    expect(analyzeGateLocation).toHaveBeenCalledWith("g-1", 5);

    const app = useAppStore();
    vi.mocked(analyzeGateLocation).mockRejectedValue(new Error("尚未生成体积网格"));
    await results.runGateLocation("g-1");
    expect(app.error?.message).toBe("尚未生成体积网格");
    expect(results.gateLocation).toEqual(report);
  });
});

describe("runFillPreview（填充预览）", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetAllMocks();
  });

  const report: FillPreviewReport = {
    field: [0, 0.5, 1],
    coveredCount: 2,
    coverageRatio: 2 / 3,
    uncoveredCells: [2],
    gateCells: [0],
    arrivalMaxMm: 8.2,
    warnings: ["存在无法从浇口充填的孤立区域：1 个单元（占比 33.3%）。"],
    basis: "图连通覆盖 + 最短流动路径到达序（启发式预览，非求解结果）",
  };

  it("以活跃方案浇口为源运行；覆盖场作为当前场", async () => {
    vi.mocked(previewFill).mockResolvedValue(report);
    const project = useProjectStore();
    project.project = {
      schemaVersion: 4,
      id: "p-1",
      name: "演示",
      createdMs: 1,
      updatedMs: 1,
      studies: [
        {
          id: "s-1",
          name: "填充",
          createdMs: 1,
          runnerElements: [
            { id: "re-1", kind: "gate", diameterMm: 2, start: [0, 0, 0], end: [1, 1, 1] },
          ],
          coolingChannels: [],
          process: null,
          materialId: null,
        },
      ],
      geometries: [],
    };
    project.activeStudyId = "s-1";

    const app = useAppStore();
    const results = useResultsStore();
    const busyDuring: (string | null)[] = [];
    vi.mocked(previewFill).mockImplementation(async () => {
      busyDuring.push(useAppStore().busy);
      return report;
    });

    await results.runFillPreview("g-1");

    expect(previewFill).toHaveBeenCalledWith("g-1", [
      { id: "re-1", kind: "gate", diameterMm: 2, start: [0, 0, 0], end: [1, 1, 1] },
    ]);
    expect(busyDuring).toEqual(["正在估算充填覆盖…"]);
    expect(results.fillPreview).toEqual(report);
    expect(results.loadedField).toEqual({
      field: "充填覆盖",
      timeDir: "—",
      timeS: 0,
      values: report.field,
      isMagnitude: false,
      complete: true,
    });
    expect(app.error).toBeNull();
  });

  it("无活跃方案时按空浇口调用；失败进全局错误", async () => {
    vi.mocked(previewFill).mockRejectedValue(new Error("需要至少一个浇口"));
    const app = useAppStore();
    const results = useResultsStore();

    await results.runFillPreview("g-1");

    expect(previewFill).toHaveBeenCalledWith("g-1", []);
    expect(app.error?.message).toBe("需要至少一个浇口");
    expect(results.fillPreview).toBeNull();
  });
});

describe("loadVectorComponents（矢量场三分量）", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetAllMocks();
  });

  const vectors = {
    field: "D",
    timeDir: "2",
    timeS: 2,
    components: [
      [0.001, -0.002, 0],
      [1.5e-4, 2.5e-5, -3.5e-5],
    ] as [number, number, number][],
    complete: true,
  };

  it("成功入状态；失败进全局错误且保留旧值", async () => {
    vi.mocked(loadVectorField).mockResolvedValue(vectors);
    const app = useAppStore();
    const results = useResultsStore();
    await results.loadVectorComponents("/case/run", "2", "D");
    expect(loadVectorField).toHaveBeenCalledWith("/case/run", "2", "D");
    expect(results.vectorField).toEqual(vectors);
    expect(app.error).toBeNull();

    vi.mocked(loadVectorField).mockRejectedValue(new Error("场文件不存在"));
    await results.loadVectorComponents("/case/run", "3", "D");
    expect(app.error?.message).toBe("场文件不存在");
    expect(results.vectorField).toEqual(vectors);
  });
});

describe("deformMesh（变形显示）", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetAllMocks();
  });

  it("成功返回变形网格；失败进全局错误并返回 null", async () => {
    const app = useAppStore();
    const results = useResultsStore();
    const deformed = { positions: [1, 2, 3], indices: [0], faceCells: [0] };
    vi.mocked(deformRenderMesh).mockResolvedValue(deformed);
    await expect(results.deformMesh("g-1", 5)).resolves.toEqual(deformed);
    expect(deformRenderMesh).toHaveBeenCalledWith("g-1", 5);
    expect(app.error).toBeNull();

    vi.mocked(deformRenderMesh).mockRejectedValue(new Error("尚未加载矢量场"));
    await expect(results.deformMesh("g-1", 5)).resolves.toBeNull();
    expect(app.error?.message).toBe("尚未加载矢量场");
  });
});

describe("loadTensorComponents（对称张量场）", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetAllMocks();
  });

  const tensor = {
    field: "sigma",
    timeDir: "2",
    timeS: 2,
    components: [[100, 0, 0, 0, 0, 0]] as [number, number, number, number, number, number][],
    magnitudes: [100, 50],
    principalAxes: [[1, 0, 0] as [number, number, number]],
    complete: true,
  };

  it("张量入状态，模量作为当前场（isMagnitude 标记）", async () => {
    vi.mocked(loadTensorField).mockResolvedValue(tensor);
    const app = useAppStore();
    const results = useResultsStore();
    await results.loadTensorComponents("/case/run", "2", "sigma");

    expect(loadTensorField).toHaveBeenCalledWith("/case/run", "2", "sigma");
    expect(results.tensorField).toEqual(tensor);
    expect(results.loadedField).toEqual({
      field: "sigma",
      timeDir: "2",
      timeS: 2,
      values: tensor.magnitudes,
      isMagnitude: true,
      complete: true,
    });
    expect(app.error).toBeNull();
  });

  it("加载失败进全局错误且保留旧值", async () => {
    vi.mocked(loadTensorField).mockResolvedValue(tensor);
    const results = useResultsStore();
    await results.loadTensorComponents("/case/run", "2", "sigma");

    const app = useAppStore();
    vi.mocked(loadTensorField).mockRejectedValue(new Error("不是对称张量场"));
    await results.loadTensorComponents("/case/run", "3", "T");
    expect(app.error?.message).toBe("不是对称张量场");
    expect(results.tensorField).toEqual(tensor);
  });
});
