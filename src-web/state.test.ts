import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { IpcUnavailableError } from "./lib/ipc";
import { addStudy, appStore, bootstrap, initialAppState, newProject, setError } from "./state";
import { getSystemInfo } from "./services/system";
import { createProject, listRecentProjects } from "./services/project";

vi.mock("./services/system", () => ({ getSystemInfo: vi.fn() }));
vi.mock("./services/project", () => ({
  createProject: vi.fn(),
  listRecentProjects: vi.fn(),
  loadProjectFile: vi.fn(),
  saveProjectFile: vi.fn(),
}));

function resetMocks(): void {
  vi.mocked(getSystemInfo).mockReset();
  vi.mocked(createProject).mockReset();
  vi.mocked(listRecentProjects).mockReset();
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
    await bootstrap();
    expect(appStore.get().info?.name).toBe("kairos");
    expect(appStore.get().recents).toHaveLength(1);
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
