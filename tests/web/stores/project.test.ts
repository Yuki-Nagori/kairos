import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { IpcUnavailableError } from "../../../src-web/utils/ipc";
import { useAppStore } from "../../../src-web/stores/app";
import { useMaterialsStore } from "../../../src-web/stores/materials";
import { useGeometryStore } from "../../../src-web/stores/geometry";
import { useResultsStore } from "../../../src-web/stores/results";
import { useProjectStore } from "../../../src-web/stores/project";
import { getSystemInfo } from "../../../src-web/api/system";
import {
  createProject,
  listRecentProjects,
  loadProjectFile,
  saveProjectFile,
  saveReportPptxToWorkspace,
  saveReportToWorkspace,
} from "../../../src-web/api/project";
import { listBuiltinMaterials, listCustomMaterials } from "../../../src-web/api/materials";
import {
  pickOpenProjectPath,
  pickSaveProjectPath,
  pickWorkspaceDir,
} from "../../../src-web/api/dialog";
import { checkMoldNetwork } from "../../../src-web/api/mold";
import type { CoolingChannel, Project, RunnerElement, Study } from "../../../src-web/types";

vi.mock("../../../src-web/api/system", () => ({ getSystemInfo: vi.fn() }));
vi.mock("../../../src-web/api/project", () => ({
  createProject: vi.fn(),
  resetProjectSession: vi.fn(),
  defaultWorkspacePath: vi.fn(async () => "/home/u/Documents/kairos"),
  saveReportToWorkspace: vi.fn(),
  saveReportPptxToWorkspace: vi.fn(),
  projectPath: vi.fn(
    async (workspace: string, name: string) => `${workspace}/${name}/${name}.kairos`,
  ),
  // 工作区根 = 工程目录的父级（`<工作区>/<工程名>/<工程名>.kairos` 布局）
  workspaceRootOf: vi.fn(async (path: string) => path.split("/").slice(0, -2).join("/") || null),
  archiveWorkspaceGeometry: vi.fn(),
  loadWorkspaceGeometry: vi.fn(),
  saveStudyMesh: vi.fn(),
  restoreStudyMesh: vi.fn(),
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
  pickWorkspaceDir: vi.fn(),
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
    geometries: [],
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
          filler: null,
          blowing: null,
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
    it("newProject 落在「<工作区>/<工程名>/」并立即落盘", async () => {
      const app = useAppStore();
      const project = useProjectStore();
      project.projectPath = "/old.kairos";
      const busyDuring: (string | null)[] = [];
      vi.mocked(createProject).mockImplementation(async (name) => {
        busyDuring.push(useAppStore().busy);
        return makeProject({ name });
      });
      const { saveProjectFile } = await import("../../../src-web/api/project");

      const created = await project.newProject("新项目", "/Volumes/Work/kairos");

      expect(created).toBe(true);
      expect(project.project?.name).toBe("新项目");
      // 工程文件与项目名同名；工作区由调用方给定
      expect(project.projectPath).toBe("/Volumes/Work/kairos/新项目/新项目.kairos");
      expect(project.workspaceRoot).toBe("/Volumes/Work/kairos");
      expect(saveProjectFile).toHaveBeenCalledWith(
        "/Volumes/Work/kairos/新项目/新项目.kairos",
        project.project,
      );
      expect(busyDuring).toEqual(["正在创建项目…"]);
      expect(app.busy).toBeNull();
    });

    it("newProject 未给工作区时留给后端默认根（工作区留空）", async () => {
      const project = useProjectStore();
      vi.mocked(createProject).mockResolvedValue(makeProject({ name: "新项目" }));
      const { projectPath: apiProjectPath } = await import("../../../src-web/api/project");
      await project.newProject("新项目");
      expect(apiProjectPath).toHaveBeenCalledWith("", "新项目");
      expect(project.projectPath).toBe("/新项目/新项目.kairos");
    });

    it("reports newProject failures and clears busy", async () => {
      vi.mocked(createProject).mockRejectedValue(new Error("磁盘不可写"));

      const app = useAppStore();
      const project = useProjectStore();
      const created = await project.newProject("新项目");

      expect(created).toBe(false);
      expect(app.error?.message).toBe("磁盘不可写");
      expect(project.project).toBeNull();
      expect(app.busy).toBeNull();
    });

    it("newProject activates the default study it comes with", async () => {
      const study = makeStudy({ id: "study-9", name: "方案 1" });
      vi.mocked(createProject).mockResolvedValue(makeProject({ studies: [study] }));

      const project = useProjectStore();
      await project.newProject("新项目");

      expect(project.activeStudyId).toBe("study-9");
      expect(project.activeStudy?.name).toBe("方案 1");
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

    it("opens a project onto its first study", async () => {
      vi.mocked(loadProjectFile).mockResolvedValue(
        makeProject({ name: "demo", studies: [makeStudy({ id: "study-3" })] }),
      );

      const project = useProjectStore();
      await project.openProjectAtPath("/p/demo.kairos");

      expect(project.activeStudyId).toBe("study-3");
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
      expect(app.error?.message).toContain("已存在同名方案");
      // 新增的方案成为活跃方案（activeStudy getter 跟随 activeStudyId）。
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

      expect(app.error?.message).toContain("请先选择一个方案");
    });

    it("requires an active study", () => {
      const app = useAppStore();
      const project = useProjectStore();
      project.project = makeProject();
      project.touchActiveStudy(() => {});

      expect(app.error?.message).toContain("请先选择一个方案");
    });

    it("applies the mutation and stamps updatedMs", () => {
      // 假计时器锁定盖章语义：updatedMs 必须等于 mutate 时刻（此前
      // toBeGreaterThanOrEqual 连「不盖章」都能通过——消融 B20 的加固点）。
      vi.useFakeTimers();
      vi.setSystemTime(1000);
      try {
        const app = useAppStore();
        const { project, study } = primeActiveStudy();

        vi.setSystemTime(1500);
        project.touchActiveStudy((target) => {
          target.name = "改名后";
        });

        expect(app.error).toBeNull();
        expect(project.activeStudy?.name).toBe("改名后");
        expect(project.project?.updatedMs).toBe(1500);
        expect(study.name).toBe("改名后");
      } finally {
        vi.useRealTimers();
      }
    });
  });

  describe("自动保存（方案配置编辑后防抖落盘）", () => {
    it("连续编辑只落盘一次，写的是最新工程", async () => {
      vi.useFakeTimers();
      try {
        vi.mocked(saveProjectFile).mockResolvedValue(undefined);
        const { project } = primeActiveStudy();
        project.projectPath = "/w/未命名项目/project.kairos";

        project.touchActiveStudy((study) => {
          study.materialId = "builtin-pp-001";
        });
        await vi.advanceTimersByTimeAsync(400); // 防抖窗口内再改一次
        project.touchActiveStudy((study) => {
          study.process = {
            meltTempC: 230,
            moldTempC: 40,
            ejectionTempC: 90,
            injectionTimeS: 1.5,
            vpSwitchVolumePercent: 96,
            packingPressureMpaCurve: [
              [0, 60],
              [8, 48],
            ],
            packingTimeS: 8,
            coolingTimeS: 15,
            coolantTempC: 25,
          };
        });
        expect(saveProjectFile).not.toHaveBeenCalled();

        await vi.advanceTimersByTimeAsync(800);
        expect(saveProjectFile).toHaveBeenCalledTimes(1);
        const [path, written] = vi.mocked(saveProjectFile).mock.calls[0] ?? [];
        expect(path).toBe("/w/未命名项目/project.kairos");
        expect(written?.studies[0]?.materialId).toBe("builtin-pp-001");
        expect(written?.studies[0]?.process?.meltTempC).toBe(230);
      } finally {
        vi.useRealTimers();
      }
    });

    it("几何引用登记同样触发自动保存", async () => {
      vi.useFakeTimers();
      try {
        vi.mocked(saveProjectFile).mockResolvedValue(undefined);
        const { project } = primeActiveStudy();
        project.projectPath = "/w/未命名项目/project.kairos";

        project.upsertGeometryRef({
          id: "geo-1",
          fileName: "part.stl",
          relativePath: "geometry/part.stl",
        });
        await vi.advanceTimersByTimeAsync(800);

        expect(saveProjectFile).toHaveBeenCalledTimes(1);
        expect(vi.mocked(saveProjectFile).mock.calls[0]?.[1]?.geometries).toHaveLength(1);
      } finally {
        vi.useRealTimers();
      }
    });

    it("没有保存路径时不排队（散装工程仍只在显式保存时落盘）", async () => {
      vi.useFakeTimers();
      try {
        const { project } = primeActiveStudy();
        project.projectPath = null;

        project.touchActiveStudy((study) => {
          study.materialId = "builtin-pp-001";
        });
        await vi.advanceTimersByTimeAsync(2000);

        expect(saveProjectFile).not.toHaveBeenCalled();
      } finally {
        vi.useRealTimers();
      }
    });

    it("自动保存失败进全局错误；缺工程 / 缺路径时直接返回", async () => {
      const app = useAppStore();
      const project = useProjectStore();
      vi.mocked(saveProjectFile).mockRejectedValue(new Error("磁盘满了"));

      await project.autoSaveNow(); // 无工程：直接返回
      expect(saveProjectFile).not.toHaveBeenCalled();

      const { project: primed } = primeActiveStudy();
      await primed.autoSaveNow(); // 有工程无路径：直接返回
      expect(saveProjectFile).not.toHaveBeenCalled();

      primed.projectPath = "/w/未命名项目/project.kairos";
      await primed.autoSaveNow();
      expect(app.error?.message).toContain("磁盘满了");
    });
  });

  describe("syncActiveStudy", () => {
    it("活跃方案失效时落到首个方案", () => {
      const project = useProjectStore();
      project.project = makeProject({ studies: [makeStudy({ id: "study-7" })] });
      project.activeStudyId = "study-old";

      project.syncActiveStudy();

      expect(project.activeStudyId).toBe("study-7");
    });

    it("活跃方案仍存在时保持选择", () => {
      const { project } = primeActiveStudy();

      project.syncActiveStudy();

      expect(project.activeStudyId).toBe("study-1");
    });

    it("无方案工程与未打开工程都清空选择", () => {
      const project = useProjectStore();
      project.project = makeProject({ studies: [] });
      project.activeStudyId = "study-1";
      project.syncActiveStudy();
      expect(project.activeStudyId).toBeNull();

      project.project = null;
      project.activeStudyId = "study-1";
      project.syncActiveStudy();
      expect(project.activeStudyId).toBeNull();
    });
  });

  describe("selectStudy", () => {
    it("切换到存在的方案；未知 id 不改变当前选择（防御分支）", () => {
      const project = useProjectStore();
      project.project = makeProject({
        studies: [makeStudy({ id: "study-1" }), makeStudy({ id: "study-2", name: "方案 B" })],
      });
      project.activeStudyId = "study-1";

      project.selectStudy("study-2");
      expect(project.activeStudyId).toBe("study-2");

      project.selectStudy("study-x");
      expect(project.activeStudyId).toBe("study-2");
    });

    it("未打开项目时安全空转", () => {
      const project = useProjectStore();
      project.selectStudy("study-1");
      expect(project.project).toBeNull();
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
      expect(app.error?.message).toContain("请先选择一个方案");
      project.addCoolingChannel(8, [0, 0, 0], [1, 0, 0], 25);
      expect(app.error?.message).toContain("请先选择一个方案");
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

      expect(app.error?.message).toContain("请先选择一个方案");
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
        massFlowRateKgS: 0.05,
        specificHeatJKgK: 4180,
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

describe("工作区：打开工程恢复与根目录刷新", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetAllMocks();
  });

  it("打开工作区工程后刷新根目录；判定失败按散装处理", async () => {
    const project = useProjectStore();
    const { loadProjectFile, workspaceRootOf } = await import("../../../src-web/api/project");
    vi.mocked(loadProjectFile).mockResolvedValue(makeProject({ name: "工作区工程" }));
    vi.mocked(workspaceRootOf).mockResolvedValue("/home/u/Documents/kairos/p");

    await project.openProjectAtPath("/home/u/Documents/kairos/p/p.kairos");
    expect(project.workspaceRoot).toBe("/home/u/Documents/kairos/p");
    expect(project.projectPath).toBe("/home/u/Documents/kairos/p/p.kairos");

    // 工作区判定抛错：按散装处理，不打断打开流程
    vi.mocked(workspaceRootOf).mockRejectedValue(new Error("无法定位文档目录"));
    await project.openProjectAtPath("/home/u/Documents/kairos/p/p.kairos");
    expect(project.workspaceRoot).toBeNull();
    expect(useAppStore().error).toBeNull();
  });

  it("刷新根目录：无路径直接清空", async () => {
    const project = useProjectStore();
    project.projectPath = null;
    await project.refreshWorkspaceRoot();
    expect(project.workspaceRoot).toBeNull();
  });

  it("upsertGeometryRef：同 id 覆盖、不同 id 追加；无工程时忽略", () => {
    const project = useProjectStore();
    project.upsertGeometryRef({ id: "g-1", fileName: "a.stl", relativePath: "geometry/a.stl" });
    expect(project.project).toBeNull();

    project.project = makeProject({ geometries: [] });
    project.upsertGeometryRef({ id: "g-1", fileName: "a.stl", relativePath: "geometry/a.stl" });
    project.upsertGeometryRef({ id: "g-2", fileName: "b.stl", relativePath: "geometry/b.stl" });
    expect(project.project?.geometries.map((entry) => entry.id)).toEqual(["g-1", "g-2"]);
    // 覆盖时保留原有顺序（列表顺序即导入顺序，避免刷新后跳位）
    project.upsertGeometryRef({ id: "g-1", fileName: "a2.stl", relativePath: "geometry/a2.stl" });
    expect(project.project?.geometries.map((entry) => entry.fileName)).toEqual(["a2.stl", "b.stl"]);
    expect(project.project?.geometries.map((entry) => entry.id)).toEqual(["g-1", "g-2"]);
  });
});

describe("报告落盘", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.mocked(saveReportToWorkspace).mockReset();
    vi.mocked(saveReportPptxToWorkspace).mockReset();
    vi.mocked(pickWorkspaceDir).mockReset();
  });

  it("有工作区时写入并回传路径", async () => {
    const project = useProjectStore();
    project.projectPath = "/w/p/p.kairos";
    project.workspaceRoot = "/w/p";
    vi.mocked(saveReportToWorkspace).mockResolvedValue("/w/p/reports/r.html");
    await expect(project.saveReport("r.html", "<html>")).resolves.toBe("/w/p/reports/r.html");
    expect(saveReportToWorkspace).toHaveBeenCalledWith("/w/p/p.kairos", "r.html", "<html>");
    expect(useAppStore().error).toBeNull();
  });

  it("无工作区时不动手也不报错（由调用方回退下载）", async () => {
    const project = useProjectStore();
    project.projectPath = "/tmp/loose.kairos";
    project.workspaceRoot = null;
    await expect(project.saveReport("r.html", "<html>")).resolves.toBeNull();
    expect(saveReportToWorkspace).not.toHaveBeenCalled();
    expect(useAppStore().error).toBeNull();
  });

  it("写入失败记全局错误并回传 null", async () => {
    const project = useProjectStore();
    project.projectPath = "/w/p/p.kairos";
    project.workspaceRoot = "/w/p";
    vi.mocked(saveReportToWorkspace).mockRejectedValue(new Error("目录只读"));
    await expect(project.saveReport("r.html", "<html>")).resolves.toBeNull();
    expect(useAppStore().error?.message).toBe("目录只读");
  });

  it("PPTX：散装工程直接报错返回，不调命令层", async () => {
    const project = useProjectStore();
    await expect(project.saveReportPptx("标题", [])).resolves.toBeNull();
    expect(saveReportPptxToWorkspace).not.toHaveBeenCalled();
    expect(useAppStore().error?.message).toContain("散装工程");
  });

  it("PPTX：命令层失败记全局错误并回传 null", async () => {
    const project = useProjectStore();
    project.project = makeProject();
    project.projectPath = "/w/p/p.kairos";
    vi.mocked(saveReportPptxToWorkspace).mockRejectedValue(new Error("工作区只读"));
    await expect(project.saveReportPptx("标题", [])).resolves.toBeNull();
    expect(useAppStore().error?.message).toBe("工作区只读");
  });

  it("PPTX：成功时回传路径，文件名带工程名", async () => {
    const project = useProjectStore();
    project.project = makeProject();
    project.projectPath = "/w/p/p.kairos";
    vi.mocked(saveReportPptxToWorkspace).mockResolvedValue("/w/p/reports/x.pptx");
    await expect(project.saveReportPptx("标题", [])).resolves.toBe("/w/p/reports/x.pptx");
    expect(saveReportPptxToWorkspace).toHaveBeenCalledWith(
      "/w/p/p.kairos",
      expect.stringContaining("-报告"),
      "标题",
      [],
    );
  });

  it("选工作区目录：取消返回 null 且不报错；选择器失败记全局错误", async () => {
    const project = useProjectStore();
    vi.mocked(pickWorkspaceDir).mockResolvedValueOnce(null);
    await expect(project.chooseWorkspaceDir("/w")).resolves.toBeNull();
    expect(useAppStore().error).toBeNull();
    expect(pickWorkspaceDir).toHaveBeenCalledWith("/w");

    vi.mocked(pickWorkspaceDir).mockRejectedValueOnce(new Error("对话框不可用"));
    await expect(project.chooseWorkspaceDir("/w")).resolves.toBeNull();
    expect(useAppStore().error?.message).toBe("对话框不可用");
  });

  it("默认工作区根取不到时静默回 null（浏览器预览不弹环境提示）", async () => {
    const project = useProjectStore();
    const { defaultWorkspacePath } = await import("../../../src-web/api/project");
    vi.mocked(defaultWorkspacePath).mockRejectedValueOnce(new Error("IPC 不可用"));
    await expect(project.defaultWorkspace()).resolves.toBeNull();
    expect(useAppStore().error).toBeNull();
  });
});

describe("工程切换事务", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetAllMocks();
  });
  it("先保存旧工程并恢复新工程，清理旧几何和结果", async () => {
    vi.useFakeTimers();
    try {
      const p = useProjectStore();
      const g = useGeometryStore();
      const results = useResultsStore();
      p.project = makeProject({ id: "A" });
      p.projectPath = "/A/A.kairos";
      g.geometries = [{ geometryId: "old" } as never];
      results.probes = [{ id: 1, nodeIndex: 0 }];
      p.scheduleAutoSave();
      const restore = vi.spyOn(g, "restoreWorkspaceContent").mockResolvedValue();
      vi.mocked(loadProjectFile).mockResolvedValue(makeProject({ id: "B" }));
      await p.openProjectAtPath("/B/B.kairos");
      await vi.advanceTimersByTimeAsync(1000);
      expect(vi.mocked(saveProjectFile).mock.calls.map((call) => call[0])).toEqual(["/A/A.kairos"]);
      expect(restore).toHaveBeenCalledOnce();
      expect(g.geometries).toEqual([]);
      expect(results.probes).toEqual([]);
      expect(p.project?.id).toBe("B");
    } finally {
      vi.useRealTimers();
    }
  });
  it("保存失败阻止切换，保留旧工程供重试", async () => {
    const p = useProjectStore();
    p.project = makeProject({ id: "A" });
    p.projectPath = "/A/A.kairos";
    vi.mocked(saveProjectFile).mockRejectedValue(new Error("磁盘满"));
    await p.openProjectAtPath("/B/B.kairos");
    expect(loadProjectFile).not.toHaveBeenCalled();
    expect(p.project?.id).toBe("A");
    expect(useAppStore().error?.message).toBe("磁盘满");
  });
  it("在途操作期间拒绝打开和新建", async () => {
    useAppStore().beginBusy("mesh");
    const p = useProjectStore();
    await p.openProjectAtPath("/B/B.kairos");
    expect(await p.newProject("B")).toBe(false);
    expect(loadProjectFile).not.toHaveBeenCalled();
    expect(createProject).not.toHaveBeenCalled();
  });
});
