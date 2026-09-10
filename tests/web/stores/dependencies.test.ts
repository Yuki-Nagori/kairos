import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAppStore } from "../../../src-web/stores/app";
import { useDependenciesStore } from "../../../src-web/stores/dependencies";
import {
  checkDependencyUpdate,
  listRuntimeDependencies,
  openDependencyPage,
} from "../../../src-web/api/dependencies";
import { downloadComponentFile, listDownloads } from "../../../src-web/api/downloads";
import type { SavedDownload, UpdateCheck } from "../../../src-web/types";

vi.mock("../../../src-web/api/dependencies", () => ({
  checkDependencyUpdate: vi.fn(),
  listRuntimeDependencies: vi.fn(),
  openDependencyPage: vi.fn(),
}));
vi.mock("../../../src-web/api/downloads", () => ({
  downloadComponentFile: vi.fn(),
  listDownloads: vi.fn(),
  getDownloadsDir: vi.fn(),
  openDownloadsDir: vi.fn(),
}));

describe("dependencies store", () => {
  const saved: SavedDownload = {
    path: "/downloads/gmsh.tgz",
    fileName: "gmsh.tgz",
    sizeBytes: 1024,
    extractDir: "/downloads/gmsh",
    releaseTag: null,
  };

  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetAllMocks();
    // 依赖分片收尾都会重扫依赖状态，默认给空结果。
    vi.mocked(listRuntimeDependencies).mockResolvedValue([]);
    vi.mocked(listDownloads).mockResolvedValue({});
  });

  it("streams download progress then clears the stage on success", async () => {
    vi.mocked(downloadComponentFile).mockImplementation(async (_id, _url, onProgress) => {
      onProgress(37);
      expect(useDependenciesStore().componentStages.gmsh).toEqual({
        stage: "downloading",
        percent: 37,
      });
      return saved;
    });

    const deps = useDependenciesStore();
    deps.updateChecks = {
      gmsh: {
        componentId: "gmsh",
        installedTag: "v0.1.1",
        latestTag: "v0.1.1",
        updateAvailable: false,
      },
    };

    await deps.downloadComponent("gmsh", "https://gmsh.info/a.tgz");

    expect(deps.componentStages.gmsh).toBeUndefined();
    expect(deps.savedDownloads.gmsh?.fileName).toBe("gmsh.tgz");
    expect(deps.downloadedFiles.gmsh).toMatchObject({
      fileName: "gmsh.tgz",
      sizeBytes: 1024,
      extractDir: "/downloads/gmsh",
    });
    // 重新下载成功后，过期的更新检查结果被清除
    expect(deps.updateChecks.gmsh).toBeUndefined();
    // 求解环境为预编译 bundle：下载不触发任何编译。
  });

  it("ignores progress arriving after the stage has been cleared", async () => {
    vi.mocked(downloadComponentFile).mockImplementation(async (_id, _url, onProgress) => {
      const deps = useDependenciesStore();
      deps.setStage("gmsh", null); // 模拟阶段已收尾后迟到的进度
      onProgress(80);
      expect(deps.componentStages.gmsh).toBeUndefined();
      return saved;
    });

    const deps = useDependenciesStore();
    await deps.downloadComponent("gmsh", "https://gmsh.info/a.tgz");

    expect(deps.componentStages.gmsh).toBeUndefined();
    expect(deps.savedDownloads.gmsh?.fileName).toBe("gmsh.tgz");
  });

  it("stores the update check result for the component", async () => {
    const check: UpdateCheck = {
      componentId: "moldingfoam",
      installedTag: "v0.1.1",
      latestTag: "v0.2.0",
      updateAvailable: true,
    };
    vi.mocked(checkDependencyUpdate).mockResolvedValue(check);

    const deps = useDependenciesStore();
    await deps.checkUpdate("moldingfoam");

    expect(deps.updateChecks.moldingfoam).toEqual(check);
    expect(deps.updateChecks.moldingfoam?.updateAvailable).toBe(true);
  });

  it("reports update check failures into the app store", async () => {
    vi.mocked(checkDependencyUpdate).mockRejectedValue(new Error("网络不可达"));

    const app = useAppStore();
    const deps = useDependenciesStore();
    await deps.checkUpdate("moldingfoam");

    expect(app.error?.message).toBe("网络不可达");
    expect(deps.updateChecks.moldingfoam).toBeUndefined();
  });

  it("lands a failed download in the failed stage with the reason", async () => {
    vi.mocked(downloadComponentFile).mockRejectedValue(new Error("网络中断"));

    const deps = useDependenciesStore();
    await deps.downloadComponent("gmsh", "https://gmsh.info/a.tgz");

    expect(deps.componentStages.gmsh).toEqual({
      stage: "failed",
      error: "下载失败：网络中断",
    });
  });

  it("stringifies non-Error rejections in the failed stage", async () => {
    vi.mocked(downloadComponentFile).mockRejectedValue("直链 404");

    const deps = useDependenciesStore();
    await deps.downloadComponent("gmsh", "https://gmsh.info/a.tgz");

    expect(deps.componentStages.gmsh).toEqual({
      stage: "failed",
      error: "下载失败：直链 404",
    });
  });

  it("refresh merges the dependency list with the cross-session manifest", async () => {
    vi.mocked(listRuntimeDependencies).mockResolvedValue([
      {
        id: "gmsh",
        name: "Gmsh",
        license: "GPL-2.0",
        licenseKind: "gpl",
        strategy: "direct_download",
        pageUrl: "https://gmsh.info",
        required: true,
        checkCommand: "gmsh --version",
        hint: "",
        download: { macos: "https://a", windows: "https://b", linux: "https://c" },
        ready: false,
        managedReady: true,
        updatable: true,
      },
    ]);
    vi.mocked(listDownloads).mockResolvedValue({
      gmsh: {
        fileName: "gmsh.tgz",
        sizeBytes: 1024,
        downloadedAtMs: 7,
        extractDir: "/downloads/gmsh",
      },
    });

    const app = useAppStore();
    const deps = useDependenciesStore();
    await deps.refreshDependencies();

    expect(deps.dependencies).toHaveLength(1);
    expect(deps.dependencies[0]?.ready).toBe(false);
    expect(deps.downloadedFiles.gmsh?.downloadedAtMs).toBe(7);
    expect(app.error).toBeNull();
  });

  it("reports refresh failures into the app store", async () => {
    vi.mocked(listDownloads).mockRejectedValue(new Error("清单读取失败"));

    const app = useAppStore();
    const deps = useDependenciesStore();
    await deps.refreshDependencies();

    expect(app.error?.message).toBe("清单读取失败");
    expect(deps.dependencies).toEqual([]);
  });

  it("opens the dependency page", async () => {
    vi.mocked(openDependencyPage).mockResolvedValue(undefined);

    const app = useAppStore();
    const deps = useDependenciesStore();
    await deps.openDependencyPage("https://gmsh.info");

    expect(openDependencyPage).toHaveBeenCalledWith("https://gmsh.info");
    expect(app.error).toBeNull();
  });

  it("reports openDependencyPage failures", async () => {
    vi.mocked(openDependencyPage).mockRejectedValue(new Error("无法打开浏览器"));

    const app = useAppStore();
    const deps = useDependenciesStore();
    await deps.openDependencyPage("https://gmsh.info");

    expect(app.error?.message).toBe("无法打开浏览器");
  });

  it("setStage(null) removes the entry, setStage(value) writes it", () => {
    const deps = useDependenciesStore();
    deps.setStage("gmsh", { stage: "downloading", percent: 5 });
    expect(deps.componentStages.gmsh).toEqual({ stage: "downloading", percent: 5 });

    deps.setStage("gmsh", null);
    expect(deps.componentStages.gmsh).toBeUndefined();
  });
});
