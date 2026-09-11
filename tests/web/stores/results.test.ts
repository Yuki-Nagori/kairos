import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAppStore } from "../../../src-web/stores/app";
import { useResultsStore } from "../../../src-web/stores/results";
import {
  deriveDifference as deriveDifferenceApi,
  deriveField as deriveFieldApi,
  listResultTimes,
  loadResultField,
} from "../../../src-web/api/results";
import type { ResultCatalog, ScalarField } from "../../../src-web/types";

vi.mock("../../../src-web/api/results", () => ({
  listResultTimes: vi.fn(),
  loadResultField: vi.fn(),
  deriveField: vi.fn(),
  deriveDifference: vi.fn(),
}));

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
    function stubDownloads(): { blobs: Blob[]; createObjectURL: ReturnType<typeof vi.fn> } {
      const blobs: Blob[] = [];
      const createObjectURL = vi.fn((blob: Blob) => {
        blobs.push(blob);
        return "blob:mock";
      });
      const FakeURL = class extends URL {
        static createObjectURL = createObjectURL;
        static revokeObjectURL = vi.fn();
      };
      vi.stubGlobal("URL", FakeURL);
      return { blobs, createObjectURL };
    }

    it("reports when no field has been loaded", () => {
      const app = useAppStore();
      const results = useResultsStore();

      results.exportFieldCsv();

      expect(app.error?.message).toContain("暂无可导出的场数据");
    });

    it("reports when the loaded field has no values", () => {
      const app = useAppStore();
      const results = useResultsStore();
      results.loadedField = makeField({ values: [] });

      results.exportFieldCsv();

      expect(app.error?.message).toContain("暂无可导出的场数据");
    });

    it("downloads a magnitude CSV named after the field and time", async () => {
      const { blobs, createObjectURL } = stubDownloads();
      const anchorSpy = vi.spyOn(document, "createElement");

      const app = useAppStore();
      const results = useResultsStore();
      results.loadedField = makeField();
      results.exportFieldCsv();

      const anchor = anchorSpy.mock.results
        .map((result) => result.value)
        .at(-1) as HTMLAnchorElement;
      expect(anchor.tagName).toBe("A");
      expect(anchor.download).toBe("p-0.100.csv");
      expect(anchor.href).toBe("blob:mock");
      expect(createObjectURL).toHaveBeenCalledTimes(1);

      const csv = await blobs[0]?.text();
      expect(csv).toBe("node,p (magnitude)\n0,1\n1,2\n2,3");
      expect(app.error).toBeNull();
    });

    it("keeps the plain header for non-magnitude fields", async () => {
      const { blobs } = stubDownloads();

      const results = useResultsStore();
      results.loadedField = makeField({ field: "T", isMagnitude: false });
      results.exportFieldCsv();

      const csv = await blobs[0]?.text();
      expect(csv).toBe("node,T\n0,1\n1,2\n2,3");
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

    it("遍历目录时间步收集探针采样，并恢复原时间步展示", async () => {
      vi.mocked(listResultTimes).mockResolvedValue({
        caseDir: "/case/run",
        times: [
          { dirName: "0", timeS: 0, fields: ["T"] },
          { dirName: "1", timeS: 1, fields: ["T"] },
        ],
      });
      vi.mocked(loadResultField).mockImplementation((_c, timeDir: string) => {
        const values = timeDir === "0" ? [10, 20] : [30, 40];
        return Promise.resolve(makeField({ field: "T", timeDir, values }));
      });

      const app = useAppStore();
      const results = useResultsStore();
      await results.loadResultsCatalog("/case/run");
      // 探针 2 的序号越界（直接注入以绕过入列校验）：采样回退为 0。
      results.probes = [makeProbe(0), makeProbe(1), makeProbe(5)];
      results.loadedField = makeField({ field: "T", timeDir: "0" });

      await results.loadProbeTimeSeries();

      // 目录 2 个时间步 → 逐个加载（外加恢复原时间步不在循环内重放）
      expect(loadResultField).toHaveBeenCalledTimes(2);
      expect(results.probeTimeSeries).toEqual([
        {
          probeId: 0,
          nodeIndex: 0,
          samples: [
            { timeS: 0, value: 10 },
            { timeS: 1, value: 30 },
          ],
        },
        {
          probeId: 1,
          nodeIndex: 1,
          samples: [
            { timeS: 0, value: 20 },
            { timeS: 1, value: 40 },
          ],
        },
        {
          probeId: 5,
          nodeIndex: 5,
          samples: [
            { timeS: 0, value: 0 },
            { timeS: 1, value: 0 },
          ],
        },
      ]);
      expect(results.probeSeriesField).toBe("T");
      expect(results.loadedField?.timeDir).toBe("0");
      expect(app.busy).toBeNull();
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
      vi.mocked(loadResultField).mockRejectedValue(new Error("时间步缺失"));

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
