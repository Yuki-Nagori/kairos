import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { IpcUnavailableError } from "./lib/ipc";
import { addStudy, appStore, bootstrap, initialAppState, newProject, setError } from "./state";
import { getSystemInfo } from "./services/system";
import { createProject, listRecentProjects } from "./services/project";
import { listBuiltinMaterials, listCustomMaterials } from "./services/materials";

vi.mock("./services/system", () => ({ getSystemInfo: vi.fn() }));
vi.mock("./services/project", () => ({
  createProject: vi.fn(),
  listRecentProjects: vi.fn(),
  loadProjectFile: vi.fn(),
  saveProjectFile: vi.fn(),
}));
vi.mock("./services/materials", () => ({
  listBuiltinMaterials: vi.fn(),
  listCustomMaterials: vi.fn(),
  importCustomMaterials: vi.fn(),
  upsertCustomMaterial: vi.fn(),
  deleteCustomMaterial: vi.fn(),
  exportMaterialsToFile: vi.fn(),
}));

function resetMocks(): void {
  vi.mocked(getSystemInfo).mockReset();
  vi.mocked(createProject).mockReset();
  vi.mocked(listRecentProjects).mockReset();
  vi.mocked(listBuiltinMaterials).mockReset();
  vi.mocked(listCustomMaterials).mockReset();
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
