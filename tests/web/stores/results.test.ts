import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAppStore } from "../../../src-web/stores/app";
import { useResultsStore } from "../../../src-web/stores/results";
import { listResultTimes, loadResultField } from "../../../src-web/api/results";
import type { ResultCatalog, ScalarField } from "../../../src-web/types";

vi.mock("../../../src-web/api/results", () => ({
  listResultTimes: vi.fn(),
  loadResultField: vi.fn(),
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

      expect(loadResultField).toHaveBeenCalledWith("/case/run", "0.100", "p");
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
});
