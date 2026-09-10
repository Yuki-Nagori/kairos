import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useDependenciesStore } from "../../../src-web/stores/dependencies";
import { checkDependencyUpdate, listRuntimeDependencies } from "../../../src-web/api/dependencies";
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
    // 重新下载成功后，过期的更新检查结果被清除
    expect(deps.updateChecks.gmsh).toBeUndefined();
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

    const deps = useDependenciesStore();
    await deps.checkUpdate("moldingfoam");

    expect(deps.updateChecks.moldingfoam).toEqual(check);
    expect(deps.updateChecks.moldingfoam?.updateAvailable).toBe(true);
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
});
