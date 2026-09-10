import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { IpcUnavailableError } from "../../../src-web/utils/ipc";
import { useAppStore } from "../../../src-web/stores/app";
import { useMaterialsStore } from "../../../src-web/stores/materials";
import { useProjectStore } from "../../../src-web/stores/project";
import { getSystemInfo } from "../../../src-web/api/system";
import {
  createProject,
  listRecentProjects,
  loadProjectFile,
  saveProjectFile,
} from "../../../src-web/api/project";
import { listBuiltinMaterials, listCustomMaterials } from "../../../src-web/api/materials";
import { pickOpenProjectPath, pickSaveProjectPath } from "../../../src-web/api/dialog";
import { checkMoldNetwork } from "../../../src-web/api/mold";
import type { CoolingChannel, Project, RunnerElement, Study } from "../../../src-web/types";

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
vi.mock("../../../src-web/api/dialog", () => ({
  pickOpenProjectPath: vi.fn(),
  pickSaveProjectPath: vi.fn(),
  pickOpenJsonPath: vi.fn(),
  pickExportJsonPath: vi.fn(),
  pickStlPath: vi.fn(),
}));
vi.mock("../../../src-web/api/mold", () => ({ checkMoldNetwork: vi.fn() }));

function makeStudy(overrides: Partial<Study> = {}): Study {
  return {
    id: "study-1",
    name: "填充分析",
    createdMs: 1,
    runnerElements: [],
    coolingChannels: [],
    process: null,
    materialId: null,
    ...overrides,
  };
}

function makeProject(overrides: Partial<Project> = {}): Project {
  return {
    schemaVersion: 1,
    id: "p-1",
    name: "项目",
    createdMs: 1,
    updatedMs: 1,
    studies: [],
    ...overrides,
  };
}

function primeActiveStudy(): { project: ReturnType<typeof useProjectStore>; study: Study } {
  const project = useProjectStore();
  const study = makeStudy();
  project.project = makeProject({ studies: [study] });
  project.activeStudyId = study.id;
  return { project, study };
}

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
      const app = useAppStore();
      const project = useProjectStore();
      project.projectPath = "/old.kairos";
      const busyDuring: (string | null)[] = [];
      vi.mocked(createProject).mockImplementation(async (name) => {
        busyDuring.push(useAppStore().busy);
        return makeProject({ name });
      });
      await project.newProject("新项目");

      expect(project.project?.name).toBe("新项目");
      expect(project.projectPath).toBeNull();
      expect(busyDuring).toEqual(["正在创建项目…"]);
      expect(app.busy).toBeNull();
    });

    it("reports newProject failures and clears busy", async () => {
      vi.mocked(createProject).mockRejectedValue(new Error("磁盘不可写"));

      const app = useAppStore();
      const project = useProjectStore();
      await project.newProject("新项目");

      expect(app.error?.message).toBe("磁盘不可写");
      expect(project.project).toBeNull();
      expect(app.busy).toBeNull();
    });

    it("opens the project picked by the dialog", async () => {
      vi.mocked(pickOpenProjectPath).mockResolvedValue("/p/demo.kairos");
      vi.mocked(loadProjectFile).mockResolvedValue(makeProject({ name: "demo" }));

      const app = useAppStore();
      const project = useProjectStore();
      await project.openProject();

      expect(loadProjectFile).toHaveBeenCalledWith("/p/demo.kairos");
      expect(project.project?.name).toBe("demo");
      expect(project.projectPath).toBe("/p/demo.kairos");
      expect(app.busy).toBeNull();
      expect(app.error).toBeNull();
    });

    it("does nothing when the open dialog is cancelled", async () => {
      vi.mocked(pickOpenProjectPath).mockResolvedValue(null);

      const project = useProjectStore();
      await project.openProject();

      expect(loadProjectFile).not.toHaveBeenCalled();
      expect(project.project).toBeNull();
    });

    it("reports openProjectAtPath failures and clears busy", async () => {
      vi.mocked(loadProjectFile).mockRejectedValue(new Error("文件损坏"));

      const app = useAppStore();
      const project = useProjectStore();
      await project.openProjectAtPath("/p/broken.kairos");

      expect(app.error?.message).toBe("文件损坏");
      expect(project.project).toBeNull();
      expect(app.busy).toBeNull();
    });

    it("saveProject is a no-op without an open project", async () => {
      const project = useProjectStore();
      await project.saveProject();

      expect(saveProjectFile).not.toHaveBeenCalled();
      expect(pickSaveProjectPath).not.toHaveBeenCalled();
    });

    it("saveProject writes to the known path and refreshes recents", async () => {
      vi.mocked(saveProjectFile).mockResolvedValue(undefined);
      vi.mocked(listRecentProjects).mockResolvedValue([
        { path: "/p/demo.kairos", name: "demo", lastOpenedMs: 2 },
      ]);

      const app = useAppStore();
      const project = useProjectStore();
      project.project = makeProject({ name: "demo" });
      project.projectPath = "/p/demo.kairos";
      await project.saveProject();

      expect(saveProjectFile).toHaveBeenCalledWith("/p/demo.kairos", project.project);
      expect(project.recents).toHaveLength(1);
      expect(app.busy).toBeNull();
      expect(app.error).toBeNull();
    });

    it("saveProject falls back to save-as when the path is unknown", async () => {
      vi.mocked(saveProjectFile).mockResolvedValue(undefined);
      vi.mocked(listRecentProjects).mockResolvedValue([]);
      vi.mocked(pickSaveProjectPath).mockResolvedValue("/p/未命名项目.kairos");

      const project = useProjectStore();
      project.project = makeProject({ name: "未命名项目" });
      await project.saveProject();

      expect(pickSaveProjectPath).toHaveBeenCalledWith("未命名项目");
      expect(saveProjectFile).toHaveBeenCalledTimes(1);
      expect(project.projectPath).toBe("/p/未命名项目.kairos");
    });

    it("saveProject does nothing when the save-as dialog is cancelled", async () => {
      vi.mocked(pickSaveProjectPath).mockResolvedValue(null);

      const project = useProjectStore();
      project.project = makeProject();
      await project.saveProject();

      expect(saveProjectFile).not.toHaveBeenCalled();
      expect(project.projectPath).toBeNull();
    });

    it("saveProjectAs forces the path dialog", async () => {
      vi.mocked(saveProjectFile).mockResolvedValue(undefined);
      vi.mocked(listRecentProjects).mockResolvedValue([]);
      vi.mocked(pickSaveProjectPath).mockResolvedValue("/p/copy.kairos");

      const project = useProjectStore();
      project.project = makeProject({ name: "项目" });
      project.projectPath = "/p/old.kairos";
      await project.saveProjectAs();

      expect(pickSaveProjectPath).toHaveBeenCalledWith("项目");
      expect(project.projectPath).toBe("/p/copy.kairos");
    });

    it("saveProjectAs is a no-op without a project or a cancelled dialog", async () => {
      const project = useProjectStore();
      await project.saveProjectAs();
      expect(pickSaveProjectPath).not.toHaveBeenCalled();

      project.project = makeProject();
      vi.mocked(pickSaveProjectPath).mockResolvedValue(null);
      await project.saveProjectAs();
      expect(saveProjectFile).not.toHaveBeenCalled();
    });

    it("reports writeProject failures and keeps the previous path", async () => {
      vi.mocked(saveProjectFile).mockRejectedValue(new Error("写入失败"));

      const app = useAppStore();
      const project = useProjectStore();
      project.project = makeProject();
      project.projectPath = "/p/old.kairos";
      await project.saveProject();

      expect(app.error?.message).toBe("写入失败");
      expect(project.projectPath).toBe("/p/old.kairos");
      expect(app.busy).toBeNull();
    });

    it("reports refreshRecents failures", async () => {
      vi.mocked(listRecentProjects).mockRejectedValue(new Error("读取最近项目失败"));

      const app = useAppStore();
      const project = useProjectStore();
      await project.refreshRecents();

      expect(app.error?.message).toBe("读取最近项目失败");
    });
  });

  describe("addStudy", () => {
    it("validates name and appends", () => {
      const app = useAppStore();
      const project = useProjectStore();
      project.project = makeProject();

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

    it("requires an open project first", () => {
      const app = useAppStore();
      const project = useProjectStore();
      project.addStudy("填充分析");

      expect(app.error?.message).toContain("请先新建或打开项目");
      expect(project.project).toBeNull();
    });
  });

  describe("touchActiveStudy", () => {
    it("requires an open project", () => {
      const app = useAppStore();
      const project = useProjectStore();
      project.touchActiveStudy(() => {});

      expect(app.error?.message).toContain("请先选择一个研究");
    });

    it("requires an active study", () => {
      const app = useAppStore();
      const project = useProjectStore();
      project.project = makeProject();
      project.touchActiveStudy(() => {});

      expect(app.error?.message).toContain("请先选择一个研究");
    });

    it("applies the mutation and stamps updatedMs", () => {
      const app = useAppStore();
      const { project, study } = primeActiveStudy();
      const before = project.project?.updatedMs ?? 0;

      project.touchActiveStudy((target) => {
        target.name = "改名后";
      });

      expect(app.error).toBeNull();
      expect(project.activeStudy?.name).toBe("改名后");
      expect(project.project?.updatedMs).toBeGreaterThanOrEqual(before);
      expect(study.name).toBe("改名后");
    });
  });

  describe("runner / cooling element editing", () => {
    it("adds and removes runner elements on the active study", () => {
      const { project } = primeActiveStudy();

      project.addRunnerElement("runner", 5, [0, 0, 0], [10, 0, 0]);
      project.addRunnerElement("gate", 2, [10, 0, 0], [12, 0, 0]);
      const elements = project.activeStudy?.runnerElements ?? [];
      expect(elements).toHaveLength(2);
      expect(elements[0]).toMatchObject({ kind: "runner", diameterMm: 5 });
      expect(elements[1]?.kind).toBe("gate");

      project.removeRunnerElement(elements[0]?.id ?? "");
      expect(project.activeStudy?.runnerElements).toHaveLength(1);
      expect(project.activeStudy?.runnerElements[0]?.kind).toBe("gate");
    });

    it("adds and removes cooling channels on the active study", () => {
      const { project } = primeActiveStudy();

      project.addCoolingChannel(8, [0, 20, 0], [100, 20, 0], 25);
      const channels = project.activeStudy?.coolingChannels ?? [];
      expect(channels).toHaveLength(1);
      expect(channels[0]).toMatchObject({ diameterMm: 8, inletTempC: 25 });

      project.removeCoolingChannel(channels[0]?.id ?? "");
      expect(project.activeStudy?.coolingChannels).toHaveLength(0);
    });

    it("routes element edits through touchActiveStudy validation", () => {
      const app = useAppStore();
      const project = useProjectStore();
      project.addRunnerElement("runner", 5, [0, 0, 0], [1, 0, 0]);
      expect(app.error?.message).toContain("请先选择一个研究");
      project.addCoolingChannel(8, [0, 0, 0], [1, 0, 0], 25);
      expect(app.error?.message).toContain("请先选择一个研究");
      project.removeRunnerElement("re-x");
      project.removeCoolingChannel("cc-x");
      expect(project.project).toBeNull();
    });
  });

  describe("checkNetwork", () => {
    it("requires an active study", async () => {
      const app = useAppStore();
      const project = useProjectStore();
      await project.checkNetwork();

      expect(app.error?.message).toContain("请先选择一个研究");
      expect(checkMoldNetwork).not.toHaveBeenCalled();
    });

    it("stores the mold issues on success", async () => {
      const runner: RunnerElement = {
        id: "re-1",
        kind: "runner",
        diameterMm: 5,
        start: [0, 0, 0],
        end: [10, 0, 0],
      };
      const channel: CoolingChannel = {
        id: "cc-1",
        diameterMm: 8,
        start: [0, 20, 0],
        end: [100, 20, 0],
        inletTempC: 25,
      };
      vi.mocked(checkMoldNetwork).mockResolvedValue(["水路与流道不相交"]);
      const { project, study } = primeActiveStudy();
      study.runnerElements = [runner];
      study.coolingChannels = [channel];

      await project.checkNetwork();

      expect(checkMoldNetwork).toHaveBeenCalledWith([runner], [channel]);
      expect(project.moldIssues).toEqual(["水路与流道不相交"]);
    });

    it("reports network check failures", async () => {
      vi.mocked(checkMoldNetwork).mockRejectedValue(new Error("校验崩溃"));
      const app = useAppStore();
      primeActiveStudy();

      await useProjectStore().checkNetwork();

      expect(app.error?.message).toBe("校验崩溃");
    });
  });
});
