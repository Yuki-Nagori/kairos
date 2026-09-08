import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { IpcUnavailableError } from "../../src-web/lib/ipc";
import {
  addStudy,
  appStore,
  bootstrap,
  compileDependencyAction,
  downloadComponent,
  initialAppState,
  newProject,
  setError,
} from "../../src-web/state";
import { getSystemInfo } from "../../src-web/services/system";
import { createProject, listRecentProjects } from "../../src-web/services/project";
import { listBuiltinMaterials, listCustomMaterials } from "../../src-web/services/materials";
import { compileDependency, listRuntimeDependencies } from "../../src-web/services/dependencies";
import { downloadComponentFile, listDownloads } from "../../src-web/services/downloads";
import type { SavedDownload } from "../../src-web/types";

vi.mock("../../src-web/services/system", () => ({ getSystemInfo: vi.fn() }));
vi.mock("../../src-web/services/project", () => ({
  createProject: vi.fn(),
  listRecentProjects: vi.fn(),
  loadProjectFile: vi.fn(),
  saveProjectFile: vi.fn(),
}));
vi.mock("../../src-web/services/geometry", () => ({
  importStl: vi.fn(),
  removeGeometry: vi.fn(),
}));
vi.mock("../../src-web/services/materials", () => ({
  listBuiltinMaterials: vi.fn(),
  listCustomMaterials: vi.fn(),
  importCustomMaterials: vi.fn(),
  upsertCustomMaterial: vi.fn(),
  deleteCustomMaterial: vi.fn(),
  exportMaterialsToFile: vi.fn(),
}));
vi.mock("../../src-web/services/dependencies", () => ({
  compileDependency: vi.fn(),
  listRuntimeDependencies: vi.fn(),
  openDependencyPage: vi.fn(),
}));
vi.mock("../../src-web/services/downloads", () => ({
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
  vi.mocked(compileDependency).mockReset();
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

    await downloadComponent("gmsh", "https://gmsh.info/a.tgz");

    expect(appStore.get().componentStages.gmsh).toBeUndefined();
    expect(appStore.get().savedDownloads.gmsh?.fileName).toBe("gmsh.tgz");
    // 非源码组件不触发编译
    expect(compileDependency).not.toHaveBeenCalled();
  });

  it("lands a failed download in the failed stage with the reason", async () => {
    vi.mocked(downloadComponentFile).mockRejectedValue(new Error("网络中断"));

    await downloadComponent("gmsh", "https://gmsh.info/a.tgz");

    expect(appStore.get().componentStages.gmsh).toEqual({
      stage: "failed",
      error: "下载失败：网络中断",
      logs: [],
    });
  });

  it("openfoam download auto-compiles with logs, then clears on success", async () => {
    vi.mocked(downloadComponentFile).mockResolvedValue(saved);
    let emitLog: ((line: string) => void) | undefined;
    let release: ((message: string) => void) | undefined;
    vi.mocked(compileDependency).mockImplementation(
      (_id, onLog) =>
        new Promise((resolve) => {
          emitLog = onLog;
          release = resolve;
        }),
    );

    await downloadComponent("openfoam", "https://github.com/OpenFOAM/a.tar.gz");
    expect(compileDependency).toHaveBeenCalledWith("openfoam", expect.any(Function));
    emitLog?.("wmake 1/3");
    expect(appStore.get().componentStages.openfoam).toEqual({
      stage: "compiling",
      logs: ["wmake 1/3"],
    });

    release?.("编译完成");
    await vi.waitFor(() => expect(appStore.get().componentStages.openfoam).toBeUndefined());
  });

  it("keeps the ring buffer tail (last 200 lines) when compiling fails", async () => {
    vi.mocked(compileDependency).mockImplementation(async (_id, onLog) => {
      for (let i = 0; i < 250; i += 1) {
        onLog(`line-${i}`);
      }
      throw new Error("退出码 2");
    });

    await compileDependencyAction("openfoam");

    const stage = appStore.get().componentStages.openfoam;
    expect(stage?.stage).toBe("failed");
    if (stage?.stage !== "failed") {
      return;
    }
    expect(stage.error).toBe("编译失败：退出码 2");
    expect(stage.logs).toHaveLength(200);
    expect(stage.logs[0]).toBe("line-50");
    expect(stage.logs.at(-1)).toBe("line-249");
  });

  it("ignores re-entrant compile calls while one is running", async () => {
    let release: ((message: string) => void) | undefined;
    vi.mocked(compileDependency).mockImplementation(
      () =>
        new Promise((resolve) => {
          release = resolve;
        }),
    );

    const first = compileDependencyAction("openfoam");
    await compileDependencyAction("openfoam");
    expect(compileDependency).toHaveBeenCalledTimes(1);

    release?.("编译完成");
    await first;
  });
});
