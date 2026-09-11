import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAppStore } from "../../../src-web/stores/app";
import { useGeometryStore } from "../../../src-web/stores/geometry";
import { useProjectStore } from "../../../src-web/stores/project";
import {
  generateDualDomainMesh,
  generateGmshMesh,
  generateMidplaneMesh,
  generateVolumeMesh,
  getRenderMesh,
  importIges,
  importSampleBox,
  importStl,
  importStep,
  repairGeometry,
  removeGeometry,
} from "../../../src-web/api/geometry";
import { pickOpenGeometryPath } from "../../../src-web/api/dialog";
import type {
  DualDomainReport,
  GeometrySummary,
  MeshingReport,
  MidplaneReport,
  RunnerElement,
  Study,
} from "../../../src-web/types";

vi.mock("../../../src-web/api/geometry", () => ({
  importStl: vi.fn(),
  importStep: vi.fn(),
  importIges: vi.fn(),
  repairGeometry: vi.fn(),
  removeGeometry: vi.fn(),
  generateVolumeMesh: vi.fn(),
  importSampleBox: vi.fn(),
  generateGmshMesh: vi.fn(),
  generateDualDomainMesh: vi.fn(),
  generateMidplaneMesh: vi.fn(),
  getRenderMesh: vi.fn(),
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

function makeMidplaneReport(): MidplaneReport {
  return {
    nodeCount: 8,
    elementCount: 4,
    beamCount: 0,
    couplingCount: 0,
    uncoupledEndpoints: 0,
    unpairedVertices: 0,
    droppedElements: 0,
    thicknessMin: 2,
    thicknessMax: 2,
    thicknessAvg: 2,
  };
}

function makeDualReport(): DualDomainReport {
  return {
    nodeCount: 8,
    triangleCount: 12,
    beamCount: 1,
    couplingCount: 1,
    uncoupledEndpoints: 1,
    unpairedTriangles: 0,
    thicknessMin: 2,
    thicknessMax: 2,
    thicknessAvg: 2,
  };
}

function studyWithRunners(runners: RunnerElement[]): Study {
  return {
    id: "study-1",
    name: "填充分析",
    createdMs: 1,
    runnerElements: runners,
    coolingChannels: [],
    process: null,
    materialId: null,
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

    it.each(["igs", "iges"])("%S 文件分派到 IGES 镶嵌导入", async (extension) => {
      vi.mocked(pickOpenGeometryPath).mockResolvedValue(`/models/壳体.${extension}`);
      const summary = makeSummary();
      vi.mocked(importIges).mockResolvedValue(summary);

      const geometry = useGeometryStore();
      await geometry.importGeometry();

      expect(importIges).toHaveBeenCalledWith(`/models/壳体.${extension}`);
      expect(importStl).not.toHaveBeenCalled();
      expect(importStep).not.toHaveBeenCalled();
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

  describe("fetchRenderMesh", () => {
    it("透传渲染网格数据", async () => {
      const data = { positions: [0, 0, 0], indices: [0, 1, 2], faceCells: [0] };
      vi.mocked(getRenderMesh).mockResolvedValue(data);
      const geometry = useGeometryStore();
      await expect(geometry.fetchRenderMesh("g-1")).resolves.toEqual(data);
      expect(getRenderMesh).toHaveBeenCalledWith("g-1");
      expect(useAppStore().error).toBeNull();
    });

    it("失败进全局错误并返回 undefined", async () => {
      vi.mocked(getRenderMesh).mockRejectedValue(new Error("几何不存在"));
      const app = useAppStore();
      const geometry = useGeometryStore();
      await expect(geometry.fetchRenderMesh("g-x")).resolves.toBeUndefined();
      expect(app.error?.message).toBe("几何不存在");
    });
  });

  describe("repairGeometryById", () => {
    it("刷新几何摘要、记录修复报告、作废体积网格并清 busy", async () => {
      const repaired = makeSummary("g-1");
      const repairReport = {
        mergedVertices: 3,
        removedDegenerate: 1,
        filledHoles: 2,
        filledTriangles: 4,
        flippedFaces: 5,
        selfIntersections: 0,
      };
      vi.mocked(repairGeometry).mockResolvedValue({ summary: repaired, report: repairReport });

      const geometry = useGeometryStore();
      const app = useAppStore();
      geometry.geometries = [makeSummary("g-1"), makeSummary("g-2")];
      geometry.meshReports["g-1"] = {
        engine: "voxel",
        nodeCount: 100,
        elementCount: 5000,
        surfaceFaceCount: 120,
        totalVolume: 1000,
        quality: { minEdgeRatio: 0.4, avgEdgeRatio: 0.8, maxEdgeRatio: 1.2, minVolume: 0.01 },
      };

      await geometry.repairGeometryById("g-1");

      expect(repairGeometry).toHaveBeenCalledWith("g-1");
      expect(geometry.geometries[0]?.triangleCount).toBe(12);
      // 非目标几何不受影响
      expect(geometry.geometries.map((g) => g.geometryId)).toEqual(["g-1", "g-2"]);
      expect(geometry.meshReports["g-1"]).toBeUndefined();
      expect(geometry.repairReports["g-1"]).toEqual(repairReport);
      expect(app.busy).toBeNull();
    });

    it("修复失败时错误进入全局状态", async () => {
      const app = useAppStore();
      const geometry = useGeometryStore();
      geometry.geometries = [makeSummary("g-1")];
      vi.mocked(repairGeometry).mockRejectedValue(new Error("修复失败"));

      await geometry.repairGeometryById("g-1");
      expect(app.error?.message).toBe("修复失败");
      expect(app.busy).toBeNull();
    });

    it("修复失败不影响既有体积网格报告", async () => {
      const app = useAppStore();
      const geometry = useGeometryStore();
      geometry.geometries = [makeSummary("g-1")];
      geometry.meshReports["g-1"] = {
        engine: "voxel",
        nodeCount: 100,
        elementCount: 5000,
        surfaceFaceCount: 120,
        totalVolume: 1000,
        quality: { minEdgeRatio: 0.4, avgEdgeRatio: 0.8, maxEdgeRatio: 1.2, minVolume: 0.01 },
      };
      vi.mocked(repairGeometry).mockRejectedValueOnce(new Error("修复失败"));
      await geometry.repairGeometryById("g-1");

      expect(app.error?.message).toBe("修复失败");
      // 既有体积网格报告不被波及
      expect(geometry.meshReports["g-1"]).toBeDefined();
    });

    it("修复成功同时作废双域网格报告", async () => {
      const geometry = useGeometryStore();
      geometry.geometries = [makeSummary("g-1")];
      geometry.dualDomainReports["g-1"] = makeDualReport();
      vi.mocked(repairGeometry).mockResolvedValue({
        summary: makeSummary("g-1"),
        report: {
          mergedVertices: 0,
          removedDegenerate: 0,
          filledHoles: 0,
          filledTriangles: 0,
          flippedFaces: 0,
          selfIntersections: 0,
        },
      });

      await geometry.repairGeometryById("g-1");

      expect(geometry.dualDomainReports["g-1"]).toBeUndefined();
    });
  });

  describe("generateDualDomain", () => {
    it("无活跃方案时以空杆系调用并保存报告", async () => {
      const report = makeDualReport();
      vi.mocked(generateDualDomainMesh).mockResolvedValue(report);

      const app = useAppStore();
      const geometry = useGeometryStore();
      await geometry.generateDualDomain("g-1");

      expect(generateDualDomainMesh).toHaveBeenCalledWith("g-1", []);
      expect(geometry.dualDomainReports["g-1"]).toEqual(report);
      expect(app.busy).toBeNull();
      expect(app.error).toBeNull();
    });

    it("活跃方案的流道/浇口作为杆系透传", async () => {
      const project = useProjectStore();
      const runners: RunnerElement[] = [
        {
          id: "r-1",
          kind: "runner",
          diameterMm: 5,
          start: [0, 0, 0],
          end: [10, 0, 0],
        },
      ];
      project.project = {
        schemaVersion: 4,
        id: "p-1",
        name: "演示",
        createdMs: 1,
        updatedMs: 1,
        studies: [studyWithRunners(runners)],
      };
      project.activeStudyId = "study-1";
      vi.mocked(generateDualDomainMesh).mockResolvedValue(makeDualReport());

      const geometry = useGeometryStore();
      await geometry.generateDualDomain("g-1");

      expect(generateDualDomainMesh).toHaveBeenCalledWith("g-1", runners);
    });

    it("生成失败时错误进入全局状态", async () => {
      const app = useAppStore();
      const geometry = useGeometryStore();
      vi.mocked(generateDualDomainMesh).mockRejectedValue(new Error("双域失败"));

      await geometry.generateDualDomain("g-1");

      expect(app.error?.message).toBe("双域失败");
      expect(geometry.dualDomainReports["g-1"]).toBeUndefined();
      expect(app.busy).toBeNull();
    });
  });

  describe("generateMidplane", () => {
    it("无活跃方案时以空杆系调用并保存报告", async () => {
      const report = makeMidplaneReport();
      vi.mocked(generateMidplaneMesh).mockResolvedValue(report);

      const app = useAppStore();
      const geometry = useGeometryStore();
      await geometry.generateMidplane("g-1");

      expect(generateMidplaneMesh).toHaveBeenCalledWith("g-1", []);
      expect(geometry.midplaneReports["g-1"]).toEqual(report);
      expect(app.busy).toBeNull();
    });

    it("活跃方案的流道/浇口作为杆系透传", async () => {
      const project = useProjectStore();
      const runners: RunnerElement[] = [
        { id: "r-1", kind: "runner", diameterMm: 5, start: [0, 0, 0], end: [10, 0, 0] },
      ];
      project.project = {
        schemaVersion: 4,
        id: "p-1",
        name: "演示",
        createdMs: 1,
        updatedMs: 1,
        studies: [studyWithRunners(runners)],
      };
      project.activeStudyId = "study-1";
      vi.mocked(generateMidplaneMesh).mockResolvedValue(makeMidplaneReport());

      const geometry = useGeometryStore();
      await geometry.generateMidplane("g-1");

      expect(generateMidplaneMesh).toHaveBeenCalledWith("g-1", runners);
    });

    it("生成失败时错误进入全局状态", async () => {
      const app = useAppStore();
      const geometry = useGeometryStore();
      vi.mocked(generateMidplaneMesh).mockRejectedValue(new Error("中面失败"));

      await geometry.generateMidplane("g-1");

      expect(app.error?.message).toBe("中面失败");
      expect(geometry.midplaneReports["g-1"]).toBeUndefined();
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

      expect(generateVolumeMesh).toHaveBeenCalledWith("g-1", 2, undefined);
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

    it("passes refinement option through to the IPC layer", async () => {
      vi.mocked(generateVolumeMesh).mockResolvedValue(makeReport());
      const geometry = useGeometryStore();

      await geometry.generateMesh("g-1", 2, { mode: "boundaryLayers", layers: 2, ratio: 0.5 });

      expect(generateVolumeMesh).toHaveBeenCalledWith("g-1", 2, {
        mode: "boundaryLayers",
        layers: 2,
        ratio: 0.5,
      });
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
