import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { IpcUnavailableError } from "../../src-web/utils/ipc";
import {
  addStudy,
  appStore,
  bootstrap,
  checkUpdateAction,
  downloadComponent,
  initialAppState,
  newProject,
  setError,
} from "../../src-web/state";
import { getSystemInfo } from "../../src-web/api/system";
import { createProject, listRecentProjects } from "../../src-web/api/project";
import { listBuiltinMaterials, listCustomMaterials } from "../../src-web/api/materials";
import { checkDependencyUpdate, listRuntimeDependencies } from "../../src-web/api/dependencies";
import { downloadComponentFile, listDownloads } from "../../src-web/api/downloads";
import type { SavedDownload, UpdateCheck } from "../../src-web/types";

vi.mock("../../src-web/api/system", () => ({ getSystemInfo: vi.fn() }));
vi.mock("../../src-web/api/project", () => ({
  createProject: vi.fn(),
  listRecentProjects: vi.fn(),
  loadProjectFile: vi.fn(),
  saveProjectFile: vi.fn(),
}));
vi.mock("../../src-web/api/geometry", () => ({
  importStl: vi.fn(),
  removeGeometry: vi.fn(),
}));
vi.mock("../../src-web/api/materials", () => ({
  listBuiltinMaterials: vi.fn(),
  listCustomMaterials: vi.fn(),
  importCustomMaterials: vi.fn(),
  upsertCustomMaterial: vi.fn(),
  deleteCustomMaterial: vi.fn(),
  exportMaterialsToFile: vi.fn(),
}));
vi.mock("../../src-web/api/dependencies", () => ({
  checkDependencyUpdate: vi.fn(),
  listRuntimeDependencies: vi.fn(),
  openDependencyPage: vi.fn(),
}));
vi.mock("../../src-web/api/downloads", () => ({
  downloadComponentFile: vi.fn(),
  listDownloads: vi.fn(),
  getDownloadsDir: vi.fn(),
  openDownloadsDir: vi.fn(),
}));

function resetMocks(): void {
  vi.mocked(getSystemInfo).mockReset();
  vi.mocked(createProject).mockReset();
  vi.mocked(listRecentProjects).mockReset();
  vi.mocked(listBuiltinMaterials).mockReset();
  vi.mocked(listCustomMaterials).mockReset();
  vi.mocked(checkDependencyUpdate).mockReset();
  vi.mocked(listRuntimeDependencies).mockReset();
  vi.mocked(downloadComponentFile).mockReset();
  vi.mocked(listDownloads).mockReset();
  // 依赖分片收尾都会重扫依赖状态，默认给空结果。
  vi.mocked(listRuntimeDependencies).mockResolvedValue([]);
  vi.mocked(listDownloads).mockResolvedValue({});
}

describe("setError", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
  });

  afterEach(() => {
    appStore.set(initialAppState);
    resetMocks();
  });

  it("classifies IPC unavailability as an info hint", () => {
    setError(new IpcUnavailableError());
    expect(appStore.get().error?.info).toBe(true);
    expect(appStore.get().error?.message).toContain("Tauri runtime");
  });

  it("classifies other errors as real failures", () => {
    setError(new Error("boom"));
    expect(appStore.get().error?.info).toBe(false);
    expect(appStore.get().error?.message).toBe("boom");
  });

  it("accepts plain string failures", () => {
    setError("字符串错误");
    expect(appStore.get().error?.message).toBe("字符串错误");
  });
});

describe("bootstrap", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
  });

  afterEach(() => {
    appStore.set(initialAppState);
    resetMocks();
  });

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
    await bootstrap();
    expect(appStore.get().info?.name).toBe("kairos");
    expect(appStore.get().recents).toHaveLength(1);
    expect(appStore.get().materials.builtin).toHaveLength(1);
    expect(appStore.get().error).toBeNull();
  });

  it("surfaces IPC unavailability as an info hint", async () => {
    vi.mocked(getSystemInfo).mockRejectedValue(new IpcUnavailableError());
    await bootstrap();
    expect(appStore.get().info).toBeNull();
    expect(appStore.get().error?.info).toBe(true);
  });
});

describe("project lifecycle", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
  });

  afterEach(() => {
    appStore.set(initialAppState);
    resetMocks();
  });

  it("newProject replaces the open project and clears its path", async () => {
    vi.mocked(createProject).mockResolvedValue({
      schemaVersion: 1,
      id: "p-1",
      name: "新项目",
      createdMs: 1,
      updatedMs: 1,
      studies: [],
    });
    appStore.set({ projectPath: "/old.kairos" });
    await newProject("新项目");
    expect(appStore.get().project?.name).toBe("新项目");
    expect(appStore.get().projectPath).toBeNull();
  });

  it("addStudy validates name and appends", () => {
    appStore.set({
      project: {
        schemaVersion: 1,
        id: "p-1",
        name: "项目",
        createdMs: 1,
        updatedMs: 1,
        studies: [],
      },
    });
    addStudy("  ");
    expect(appStore.get().error?.message).toContain("不能为空");

    addStudy("填充分析");
    addStudy("填充分析");
    const studies = appStore.get().project?.studies ?? [];
    expect(studies).toHaveLength(1);
    expect(appStore.get().error?.message).toContain("已存在同名研究");
  });
});

describe("dependency stage transitions", () => {
  const saved: SavedDownload = {
    path: "/downloads/gmsh.tgz",
    fileName: "gmsh.tgz",
    sizeBytes: 1024,
    extractDir: "/downloads/gmsh",
    releaseTag: null,
  };

  beforeEach(() => {
    appStore.set(initialAppState);
    resetMocks();
  });

  afterEach(async () => {
    appStore.set(initialAppState);
    resetMocks();
  });

  it("streams download progress then clears the stage on success", async () => {
    vi.mocked(downloadComponentFile).mockImplementation(async (_id, _url, onProgress) => {
      onProgress(37);
      expect(appStore.get().componentStages.gmsh).toEqual({
        stage: "downloading",
        percent: 37,
      });
      return saved;
    });

    appStore.set({
      updateChecks: {
        gmsh: {
          componentId: "gmsh",
          installedTag: "v0.1.1",
          latestTag: "v0.1.1",
          updateAvailable: false,
        },
      },
    });

    await downloadComponent("gmsh", "https://gmsh.info/a.tgz");

    expect(appStore.get().componentStages.gmsh).toBeUndefined();
    expect(appStore.get().savedDownloads.gmsh?.fileName).toBe("gmsh.tgz");
    // 重新下载成功后，过期的更新检查结果被清除
    expect(appStore.get().updateChecks.gmsh).toBeUndefined();
    // 求解环境为预编译 bundle：下载不触发任何编译。
  });

  it("stores the update check result for the component", async () => {
    const check: UpdateCheck = {
      componentId: "moldingfoam",
      installedTag: "v0.1.1",
      latestTag: "v0.2.0",
      updateAvailable: true,
    };
    vi.mocked(checkDependencyUpdate).mockResolvedValue(check);

    await checkUpdateAction("moldingfoam");

    expect(appStore.get().updateChecks.moldingfoam).toEqual(check);
    expect(appStore.get().updateChecks.moldingfoam?.updateAvailable).toBe(true);
  });

  it("lands a failed download in the failed stage with the reason", async () => {
    vi.mocked(downloadComponentFile).mockRejectedValue(new Error("网络中断"));

    await downloadComponent("gmsh", "https://gmsh.info/a.tgz");

    expect(appStore.get().componentStages.gmsh).toEqual({
      stage: "failed",
      error: "下载失败：网络中断",
    });
  });
});
