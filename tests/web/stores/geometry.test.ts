import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAppStore } from "../../../src-web/stores/app";
import { useGeometryStore } from "../../../src-web/stores/geometry";
import {
  generateGmshMesh,
  generateVolumeMesh,
  importSampleBox,
  importStl,
  importStep,
  removeGeometry,
} from "../../../src-web/api/geometry";
import { pickOpenGeometryPath } from "../../../src-web/api/dialog";
import type { GeometrySummary, MeshingReport } from "../../../src-web/types";

vi.mock("../../../src-web/api/geometry", () => ({
  importStl: vi.fn(),
  importStep: vi.fn(),
  removeGeometry: vi.fn(),
  generateVolumeMesh: vi.fn(),
  importSampleBox: vi.fn(),
  generateGmshMesh: vi.fn(),
}));
vi.mock("../../../src-web/api/dialog", () => ({
  pickOpenProjectPath: vi.fn(),
  pickSaveProjectPath: vi.fn(),
  pickOpenJsonPath: vi.fn(),
  pickExportJsonPath: vi.fn(),
  pickOpenGeometryPath: vi.fn(),
}));

function makeSummary(id = "g-1"): GeometrySummary {
  return {
    geometryId: id,
    fileName: "box.stl",
    triangleCount: 12,
    size: [10, 10, 10],
    surfaceArea: 600,
    signedVolume: 1000,
    suggestedUnit: "mm",
    issues: {
      degenerate: 0,
      openEdges: 0,
      nonManifoldEdges: 0,
      normalInconsistentEdges: 0,
    },
  };
}

function makeReport(): MeshingReport {
  return {
    engine: "voxel",
    nodeCount: 8,
    elementCount: 6,
    surfaceFaceCount: 12,
    totalVolume: 1000,
    quality: { minEdgeRatio: 1, avgEdgeRatio: 1.2, maxEdgeRatio: 2, minVolume: 0.5 },
  };
}

describe("geometry store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetAllMocks();
  });

  describe("importGeometry", () => {
    it("does nothing when the dialog is cancelled", async () => {
      vi.mocked(pickOpenGeometryPath).mockResolvedValue(null);

      const app = useAppStore();
      const geometry = useGeometryStore();
      await geometry.importGeometry();

      expect(importStl).not.toHaveBeenCalled();
      expect(app.busy).toBeNull();
      expect(geometry.geometries).toEqual([]);
    });

    it("appends the imported summary and clears busy", async () => {
      vi.mocked(pickOpenGeometryPath).mockResolvedValue("/models/box.stl");
      const summary = makeSummary();
      vi.mocked(importStl).mockResolvedValue(summary);

      const app = useAppStore();
      const geometry = useGeometryStore();
      const busyDuring: (string | null)[] = [];
      vi.mocked(importStl).mockImplementation(async () => {
        busyDuring.push(useAppStore().busy);
        return summary;
      });
      await geometry.importGeometry();

      expect(importStl).toHaveBeenCalledWith("/models/box.stl");
      expect(geometry.geometries).toEqual([summary]);
      expect(busyDuring).toEqual(["正在导入几何…"]);
      expect(app.busy).toBeNull();
      expect(app.error).toBeNull();
    });

    it("STEP 文件分派到镶嵌导入", async () => {
      vi.mocked(pickOpenGeometryPath).mockResolvedValue("/models/壳体.step");
      const summary = makeSummary();
      vi.mocked(importStep).mockResolvedValue(summary);

      const geometry = useGeometryStore();
      await geometry.importGeometry();

      expect(importStep).toHaveBeenCalledWith("/models/壳体.step");
      expect(importStl).not.toHaveBeenCalled();
      expect(geometry.geometries).toEqual([summary]);
    });

    it("reports import failures and clears busy", async () => {
      vi.mocked(pickOpenGeometryPath).mockResolvedValue("/models/broken.stl");
      vi.mocked(importStl).mockRejectedValue(new Error("STL 解析失败"));

      const app = useAppStore();
      const geometry = useGeometryStore();
      await geometry.importGeometry();

      expect(app.error?.message).toBe("STL 解析失败");
      expect(geometry.geometries).toEqual([]);
      expect(app.busy).toBeNull();
    });
  });

  describe("importSampleGeometry", () => {
    it("imports the sample box with the default size", async () => {
      const summary = makeSummary("sample");
      vi.mocked(importSampleBox).mockResolvedValue(summary);

      const app = useAppStore();
      const geometry = useGeometryStore();
      await geometry.importSampleGeometry();

      expect(importSampleBox).toHaveBeenCalledWith(10);
      expect(geometry.geometries).toEqual([summary]);
      expect(app.busy).toBeNull();
    });

    it("imports the sample box with a custom size", async () => {
      vi.mocked(importSampleBox).mockResolvedValue(makeSummary("sample"));
      const geometry = useGeometryStore();
      await geometry.importSampleGeometry(42);
      expect(importSampleBox).toHaveBeenCalledWith(42);
    });

    it("reports sample import failures", async () => {
      vi.mocked(importSampleBox).mockRejectedValue(new Error("样例生成失败"));

      const app = useAppStore();
      const geometry = useGeometryStore();
      await geometry.importSampleGeometry();

      expect(app.error?.message).toBe("样例生成失败");
      expect(app.busy).toBeNull();
    });
  });

  describe("removeGeometryById", () => {
    it("removes the geometry from the list", async () => {
      vi.mocked(removeGeometry).mockResolvedValue(undefined);

      const app = useAppStore();
      const geometry = useGeometryStore();
      geometry.geometries = [makeSummary("g-1"), makeSummary("g-2")];
      await geometry.removeGeometryById("g-1");

      expect(removeGeometry).toHaveBeenCalledWith("g-1");
      expect(geometry.geometries.map((g) => g.geometryId)).toEqual(["g-2"]);
      expect(app.error).toBeNull();
    });

    it("reports removal failures", async () => {
      vi.mocked(removeGeometry).mockRejectedValue(new Error("释放缓存失败"));

      const app = useAppStore();
      const geometry = useGeometryStore();
      geometry.geometries = [makeSummary("g-1")];
      await geometry.removeGeometryById("g-1");

      expect(app.error?.message).toBe("释放缓存失败");
      expect(geometry.geometries).toHaveLength(1);
    });
  });

  describe("generateMesh", () => {
    it("rejects non-positive target sizes", async () => {
      const app = useAppStore();
      const geometry = useGeometryStore();

      await geometry.generateMesh("g-1", 0);
      expect(app.error?.message).toContain("目标网格尺寸必须为正数");
      expect(app.busy).toBeNull();
      expect(generateVolumeMesh).not.toHaveBeenCalled();

      await geometry.generateMesh("g-1", -1);
      expect(app.error?.message).toContain("目标网格尺寸必须为正数");
      expect(generateVolumeMesh).not.toHaveBeenCalled();
    });

    it("stores the mesh report on success", async () => {
      const report = makeReport();
      vi.mocked(generateVolumeMesh).mockResolvedValue(report);

      const app = useAppStore();
      const geometry = useGeometryStore();
      const busyDuring: (string | null)[] = [];
      vi.mocked(generateVolumeMesh).mockImplementation(async () => {
        busyDuring.push(useAppStore().busy);
        return report;
      });
      await geometry.generateMesh("g-1", 2);

      expect(generateVolumeMesh).toHaveBeenCalledWith("g-1", 2);
      expect(geometry.meshReports["g-1"]).toEqual(report);
      expect(busyDuring).toEqual(["正在生成网格…"]);
      expect(app.busy).toBeNull();
    });

    it("reports meshing failures and clears busy", async () => {
      vi.mocked(generateVolumeMesh).mockRejectedValue(new Error("网格生成失败"));

      const app = useAppStore();
      const geometry = useGeometryStore();
      await geometry.generateMesh("g-1", 2);

      expect(app.error?.message).toBe("网格生成失败");
      expect(geometry.meshReports["g-1"]).toBeUndefined();
      expect(app.busy).toBeNull();
    });
  });

  describe("generateGmshMesh", () => {
    it("rejects non-positive target sizes", async () => {
      const app = useAppStore();
      const geometry = useGeometryStore();

      await geometry.generateGmshMesh("g-1", 0);
      expect(app.error?.message).toContain("目标网格尺寸必须为正数");
      expect(app.busy).toBeNull();
      expect(generateGmshMesh).not.toHaveBeenCalled();
    });

    it("stores the gmsh mesh report on success", async () => {
      const report: MeshingReport = { ...makeReport(), engine: "gmsh" };
      vi.mocked(generateGmshMesh).mockResolvedValue(report);

      const app = useAppStore();
      const geometry = useGeometryStore();
      await geometry.generateGmshMesh("g-1", 0.5);

      expect(generateGmshMesh).toHaveBeenCalledWith("g-1", 0.5);
      expect(geometry.meshReports["g-1"]).toEqual(report);
      expect(app.busy).toBeNull();
      expect(app.error).toBeNull();
    });

    it("reports gmsh failures and clears busy", async () => {
      vi.mocked(generateGmshMesh).mockRejectedValue(new Error("Gmsh 未就绪"));

      const app = useAppStore();
      const geometry = useGeometryStore();
      await geometry.generateGmshMesh("g-1", 0.5);

      expect(app.error?.message).toBe("Gmsh 未就绪");
      expect(app.busy).toBeNull();
    });
  });
});
