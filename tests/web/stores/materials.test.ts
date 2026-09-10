import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAppStore } from "../../../src-web/stores/app";
import { useMaterialsStore } from "../../../src-web/stores/materials";
import { useProjectStore } from "../../../src-web/stores/project";
import {
  deleteCustomMaterial,
  exportMaterialsToFile,
  importCustomMaterials,
  listBuiltinMaterials,
  listCustomMaterials,
  upsertCustomMaterial,
} from "../../../src-web/api/materials";
import { pickExportJsonPath, pickOpenJsonPath } from "../../../src-web/api/dialog";
import type { Material, Project } from "../../../src-web/types";

vi.mock("../../../src-web/api/materials", () => ({
  listBuiltinMaterials: vi.fn(),
  listCustomMaterials: vi.fn(),
  importCustomMaterials: vi.fn(),
  upsertCustomMaterial: vi.fn(),
  deleteCustomMaterial: vi.fn(),
  exportMaterialsToFile: vi.fn(),
}));
vi.mock("../../../src-web/api/dialog", () => ({
  pickOpenProjectPath: vi.fn(),
  pickSaveProjectPath: vi.fn(),
  pickOpenJsonPath: vi.fn(),
  pickExportJsonPath: vi.fn(),
  pickStlPath: vi.fn(),
}));

function makeMaterial(id: string, overrides: Partial<Material> = {}): Material {
  return {
    id,
    name: `材料-${id}`,
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
    mechanics: null,
    dataNote: "示例数据",
    ...overrides,
  };
}

function makeProject(studies: Project["studies"]): Project {
  return {
    schemaVersion: 1,
    id: "p-1",
    name: "项目",
    createdMs: 1,
    updatedMs: 1,
    studies,
  };
}

describe("materials store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetAllMocks();
  });

  describe("loadMaterials", () => {
    it("loads builtin and custom materials", async () => {
      const builtin = [makeMaterial("builtin-pp")];
      const custom = [makeMaterial("custom-1", { name: "自研配方" })];
      vi.mocked(listBuiltinMaterials).mockResolvedValue(builtin);
      vi.mocked(listCustomMaterials).mockResolvedValue(custom);

      const materials = useMaterialsStore();
      await materials.loadMaterials();

      expect(materials.materials).toEqual({ builtin, custom });
    });

    it("propagates failures to the caller (bootstrap handles them)", async () => {
      vi.mocked(listBuiltinMaterials).mockRejectedValue(new Error("IPC 掉线"));

      const materials = useMaterialsStore();
      await expect(materials.loadMaterials()).rejects.toThrow("IPC 掉线");
    });
  });

  describe("import", () => {
    it("imports materials from the picked file", async () => {
      const custom = [makeMaterial("custom-1")];
      vi.mocked(pickOpenJsonPath).mockResolvedValue("/data/custom.json");
      vi.mocked(importCustomMaterials).mockResolvedValue(custom);

      const app = useAppStore();
      const materials = useMaterialsStore();
      materials.materials = { builtin: [makeMaterial("builtin-pp")], custom: [] };
      await materials.importMaterials();

      expect(importCustomMaterials).toHaveBeenCalledWith("/data/custom.json");
      expect(materials.materials.custom).toEqual(custom);
      expect(materials.materials.builtin).toHaveLength(1);
      expect(app.busy).toBeNull();
      expect(app.error).toBeNull();
    });

    it("does nothing when the dialog is cancelled", async () => {
      vi.mocked(pickOpenJsonPath).mockResolvedValue(null);

      const materials = useMaterialsStore();
      await materials.importMaterials();

      expect(importCustomMaterials).not.toHaveBeenCalled();
    });

    it("reports import failures and clears busy", async () => {
      vi.mocked(importCustomMaterials).mockRejectedValue(new Error("JSON 格式非法"));

      const app = useAppStore();
      const materials = useMaterialsStore();
      await materials.importMaterialsFromPath("/data/bad.json");

      expect(app.error?.message).toBe("JSON 格式非法");
      expect(app.busy).toBeNull();
    });
  });

  describe("upsert / delete", () => {
    it("upserts a custom material", async () => {
      const custom = [makeMaterial("custom-1")];
      vi.mocked(upsertCustomMaterial).mockResolvedValue(custom);

      const app = useAppStore();
      const materials = useMaterialsStore();
      await materials.upsertMaterial(makeMaterial("custom-1"));

      expect(materials.materials.custom).toEqual(custom);
      expect(app.busy).toBeNull();
    });

    it("reports upsert failures", async () => {
      vi.mocked(upsertCustomMaterial).mockRejectedValue(new Error("校验不通过"));

      const app = useAppStore();
      const materials = useMaterialsStore();
      await materials.upsertMaterial(makeMaterial("bad"));

      expect(app.error?.message).toBe("校验不通过");
      expect(app.busy).toBeNull();
    });

    it("deletes a custom material", async () => {
      vi.mocked(deleteCustomMaterial).mockResolvedValue([]);

      const app = useAppStore();
      const materials = useMaterialsStore();
      materials.materials = { builtin: [], custom: [makeMaterial("custom-1")] };
      await materials.deleteMaterial("custom-1");

      expect(materials.materials.custom).toEqual([]);
      expect(app.busy).toBeNull();
    });

    it("reports delete failures", async () => {
      vi.mocked(deleteCustomMaterial).mockRejectedValue(new Error("删除失败"));

      const app = useAppStore();
      const materials = useMaterialsStore();
      await materials.deleteMaterial("custom-1");

      expect(app.error?.message).toBe("删除失败");
    });
  });

  describe("export", () => {
    it("refuses to export an empty custom library", async () => {
      const app = useAppStore();
      const materials = useMaterialsStore();
      materials.materials = { builtin: [makeMaterial("builtin-pp")], custom: [] };

      await materials.exportCustomMaterials("/data/out.json");

      expect(app.error?.message).toContain("没有可导出的自定义材料");
      expect(exportMaterialsToFile).not.toHaveBeenCalled();
      expect(app.busy).toBeNull();
    });

    it("exports custom materials to the given path", async () => {
      vi.mocked(exportMaterialsToFile).mockResolvedValue(undefined);
      const custom = [makeMaterial("custom-1")];

      const app = useAppStore();
      const materials = useMaterialsStore();
      materials.materials = { builtin: [], custom };
      await materials.exportCustomMaterials("/data/out.json");

      expect(exportMaterialsToFile).toHaveBeenCalledWith("/data/out.json", custom);
      expect(app.busy).toBeNull();
      expect(app.error).toBeNull();
    });

    it("reports export failures", async () => {
      vi.mocked(exportMaterialsToFile).mockRejectedValue(new Error("写入失败"));

      const app = useAppStore();
      const materials = useMaterialsStore();
      materials.materials = { builtin: [], custom: [makeMaterial("custom-1")] };
      await materials.exportCustomMaterials("/data/out.json");

      expect(app.error?.message).toBe("写入失败");
      expect(app.busy).toBeNull();
    });

    it("exportMaterials does nothing when the dialog is cancelled", async () => {
      vi.mocked(pickExportJsonPath).mockResolvedValue(null);

      const materials = useMaterialsStore();
      await materials.exportMaterials();

      expect(pickExportJsonPath).toHaveBeenCalledWith("kairos-custom-materials");
      expect(exportMaterialsToFile).not.toHaveBeenCalled();
    });

    it("exportMaterials delegates to the picked path", async () => {
      vi.mocked(pickExportJsonPath).mockResolvedValue("/data/out.json");
      vi.mocked(exportMaterialsToFile).mockResolvedValue(undefined);
      const custom = [makeMaterial("custom-1")];

      const materials = useMaterialsStore();
      materials.materials = { builtin: [], custom };
      await materials.exportMaterials();

      expect(exportMaterialsToFile).toHaveBeenCalledWith("/data/out.json", custom);
    });
  });

  describe("copyMaterialToCustom", () => {
    it("reports a missing source material", async () => {
      const app = useAppStore();
      const materials = useMaterialsStore();

      await materials.copyMaterialToCustom("no-such-id");

      expect(app.error?.message).toContain("未找到要复制的材料");
      expect(upsertCustomMaterial).not.toHaveBeenCalled();
    });

    it("copies a builtin material with a custom note prefix", async () => {
      const copy = makeMaterial("custom-copy");
      vi.mocked(upsertCustomMaterial).mockResolvedValue([copy]);

      const materials = useMaterialsStore();
      materials.materials = { builtin: [makeMaterial("builtin-pp")], custom: [] };
      await materials.copyMaterialToCustom("builtin-pp");

      expect(upsertCustomMaterial).toHaveBeenCalledTimes(1);
      const inserted = vi.mocked(upsertCustomMaterial).mock.calls[0]?.[0];
      expect(inserted?.id).toMatch(/^custom-\d+$/);
      expect(inserted?.name).toBe("材料-builtin-pp-副本");
      expect(inserted?.dataNote).toBe("自定义副本。示例数据");
      expect(materials.materials.custom).toEqual([copy]);
    });

    it("keeps an existing custom note untouched", async () => {
      const source = makeMaterial("custom-1", { dataNote: "自定义配方 v2" });
      vi.mocked(upsertCustomMaterial).mockResolvedValue([]);

      const materials = useMaterialsStore();
      materials.materials = { builtin: [], custom: [source] };
      await materials.copyMaterialToCustom("custom-1");

      const inserted = vi.mocked(upsertCustomMaterial).mock.calls[0]?.[0];
      expect(inserted?.dataNote).toBe("自定义配方 v2");
      expect(inserted?.name).toBe("材料-custom-1-副本");
    });
  });

  describe("assignMaterial", () => {
    it("requires an open project with an active study", () => {
      const app = useAppStore();
      const materials = useMaterialsStore();

      materials.assignMaterial("builtin-pp");
      expect(app.error?.message).toContain("请先创建或选择一个研究");

      const project = useProjectStore();
      project.project = makeProject([
        {
          id: "s-1",
          name: "填充分析",
          createdMs: 1,
          runnerElements: [],
          coolingChannels: [],
          process: null,
          materialId: null,
        },
      ]);
      materials.assignMaterial("builtin-pp");
      expect(app.error?.message).toContain("请先创建或选择一个研究");
    });

    it("reports unknown material ids", () => {
      const app = useAppStore();
      const materials = useMaterialsStore();
      const project = useProjectStore();
      project.project = makeProject([
        {
          id: "s-1",
          name: "填充分析",
          createdMs: 1,
          runnerElements: [],
          coolingChannels: [],
          process: null,
          materialId: null,
        },
      ]);
      project.activeStudyId = "s-1";
      materials.materials = { builtin: [], custom: [] };

      materials.assignMaterial("no-such-id");

      expect(app.error?.message).toContain("材料不存在");
      expect(project.project?.studies[0]?.materialId).toBeNull();
    });

    it("assigns the material only to the active study", () => {
      const materials = useMaterialsStore();
      materials.materials = {
        builtin: [makeMaterial("builtin-pp")],
        custom: [makeMaterial("custom-1")],
      };
      const project = useProjectStore();
      project.project = makeProject([
        {
          id: "s-1",
          name: "填充分析",
          createdMs: 1,
          runnerElements: [],
          coolingChannels: [],
          process: null,
          materialId: null,
        },
        {
          id: "s-2",
          name: "保压分析",
          createdMs: 2,
          runnerElements: [],
          coolingChannels: [],
          process: null,
          materialId: null,
        },
      ]);
      project.activeStudyId = "s-1";
      const before = project.project?.updatedMs ?? 0;

      materials.assignMaterial("custom-1");

      const studies = project.project?.studies ?? [];
      expect(studies[0]?.materialId).toBe("custom-1");
      expect(studies[1]?.materialId).toBeNull();
      expect(project.project?.updatedMs).toBeGreaterThanOrEqual(before);
    });
  });
});
