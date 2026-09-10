import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { IpcUnavailableError } from "../../../src-web/utils/ipc";
import { useAppStore } from "../../../src-web/stores/app";
import { useMaterialsStore } from "../../../src-web/stores/materials";
import { useProjectStore } from "../../../src-web/stores/project";
import { getSystemInfo } from "../../../src-web/api/system";
import { createProject, listRecentProjects } from "../../../src-web/api/project";
import { listBuiltinMaterials, listCustomMaterials } from "../../../src-web/api/materials";

vi.mock("../../../src-web/api/system", () => ({ getSystemInfo: vi.fn() }));
vi.mock("../../../src-web/api/project", () => ({
  createProject: vi.fn(),
  listRecentProjects: vi.fn(),
  loadProjectFile: vi.fn(),
  saveProjectFile: vi.fn(),
}));
vi.mock("../../../src-web/api/materials", () => ({
  listBuiltinMaterials: vi.fn(),
  listCustomMaterials: vi.fn(),
  importCustomMaterials: vi.fn(),
  upsertCustomMaterial: vi.fn(),
  deleteCustomMaterial: vi.fn(),
  exportMaterialsToFile: vi.fn(),
}));

describe("project store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetAllMocks();
  });

  describe("bootstrap", () => {
    it("stores app info and recents when IPC responds", async () => {
      vi.mocked(getSystemInfo).mockResolvedValue({
        name: "kairos",
        version: "0.1.0",
        os: "macos",
      });
      vi.mocked(listRecentProjects).mockResolvedValue([
        { path: "/p/demo.kairos", name: "演示项目", lastOpenedMs: 1 },
      ]);
      vi.mocked(listBuiltinMaterials).mockResolvedValue([
        {
          id: "builtin-pp-001",
          name: "PP-示例-001",
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
          dataNote: "示例数据",
        },
      ]);
      vi.mocked(listCustomMaterials).mockResolvedValue([]);

      const app = useAppStore();
      const project = useProjectStore();
      await project.bootstrap();

      expect(app.info?.name).toBe("kairos");
      expect(project.recents).toHaveLength(1);
      expect(useMaterialsStore().materials.builtin).toHaveLength(1);
      expect(app.error).toBeNull();
    });

    it("surfaces IPC unavailability as an info hint", async () => {
      vi.mocked(getSystemInfo).mockRejectedValue(new IpcUnavailableError());

      const app = useAppStore();
      const project = useProjectStore();
      await project.bootstrap();

      expect(app.info).toBeNull();
      expect(app.error?.info).toBe(true);
    });
  });

  describe("project lifecycle", () => {
    it("newProject replaces the open project and clears its path", async () => {
      vi.mocked(createProject).mockResolvedValue({
        schemaVersion: 1,
        id: "p-1",
        name: "新项目",
        createdMs: 1,
        updatedMs: 1,
        studies: [],
      });

      const project = useProjectStore();
      project.projectPath = "/old.kairos";
      await project.newProject("新项目");

      expect(project.project?.name).toBe("新项目");
      expect(project.projectPath).toBeNull();
    });

    it("addStudy validates name and appends", () => {
      const app = useAppStore();
      const project = useProjectStore();
      project.project = {
        schemaVersion: 1,
        id: "p-1",
        name: "项目",
        createdMs: 1,
        updatedMs: 1,
        studies: [],
      };

      project.addStudy("  ");
      expect(app.error?.message).toContain("不能为空");

      project.addStudy("填充分析");
      project.addStudy("填充分析");
      const studies = project.project?.studies ?? [];
      expect(studies).toHaveLength(1);
      expect(app.error?.message).toContain("已存在同名研究");
      // 新增的研究成为活跃研究（activeStudy getter 跟随 activeStudyId）。
      expect(project.activeStudyId).toBe(studies[0]?.id);
      expect(project.activeStudy?.name).toBe("填充分析");
    });
  });
});
