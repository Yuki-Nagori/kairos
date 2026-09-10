import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import DependenciesPanel from "../../../../src-web/views/dependencies/DependenciesPanel.vue";
import { useAppStore } from "../../../../src-web/stores/app";
import { useDependenciesStore } from "../../../../src-web/stores/dependencies";
import {
  checkDependencyUpdate,
  listRuntimeDependencies,
  openDependencyPage,
} from "../../../../src-web/api/dependencies";
import {
  downloadComponentFile,
  getDownloadsDir,
  listDownloads,
  openDownloadsDir,
} from "../../../../src-web/api/downloads";
import { deployVmBundle } from "../../../../src-web/api/vm";
import type {
  DependencyStatus,
  DownloadSpec,
  InstallStrategy,
  LicenseKind,
  SavedDownload,
} from "../../../../src-web/types";

vi.mock("../../../../src-web/api/dependencies", () => ({
  listRuntimeDependencies: vi.fn(),
  openDependencyPage: vi.fn(),
  checkDependencyUpdate: vi.fn(),
}));
vi.mock("../../../../src-web/api/downloads", () => ({
  downloadComponentFile: vi.fn(),
  listDownloads: vi.fn(),
  getDownloadsDir: vi.fn(),
  openDownloadsDir: vi.fn(),
}));
vi.mock("../../../../src-web/api/vm", () => ({
  getVmStatus: vi.fn(),
  installVm: vi.fn(),
  startVm: vi.fn(),
  vmShellStart: vi.fn(),
  vmShellSend: vi.fn(),
  vmShellStop: vi.fn(),
  stopVm: vi.fn(),
  deployVmBundle: vi.fn(),
}));

const download: DownloadSpec = {
  macos: "https://example.com/gmsh-macos.zip",
  windows: "https://example.com/gmsh-win.zip",
  linux: "https://example.com/gmsh-linux.tgz",
};

function makeDep(overrides: Partial<DependencyStatus> = {}): DependencyStatus {
  return {
    id: "gmsh",
    name: "Gmsh",
    license: "GPL-2.0",
    licenseKind: "gpl" satisfies LicenseKind,
    strategy: "direct_download" satisfies InstallStrategy,
    pageUrl: "https://gmsh.sh",
    required: false,
    checkCommand: "gmsh --info",
    hint: "请安装 Gmsh 后重新探测。",
    download,
    ready: false,
    managedReady: false,
    updatable: false,
    ...overrides,
  };
}

const savedFixture: SavedDownload = {
  path: "/downloads/gmsh.tgz",
  fileName: "gmsh.tgz",
  sizeBytes: 2048,
  extractDir: null,
  releaseTag: null,
};

/** 挂载面板并等探测完成（依赖行渲染出来）。 */
function findButton(wrapper: ReturnType<typeof mount>, label: string) {
  const found = wrapper.findAll("button").find((candidate) => candidate.text() === label);
  if (found === undefined) {
    throw new Error(`找不到按钮：${label}`);
  }
  return found;
}

describe("DependenciesPanel", () => {
  let pinia: Pinia;

  /** 挂载面板并等探测完成（依赖行渲染出「官方页」按钮，避免匹配卡片静态提示文案）。 */
  async function mountPanel(deps: DependencyStatus[]) {
    vi.mocked(listRuntimeDependencies).mockResolvedValue(deps);
    const wrapper = mount(DependenciesPanel, { global: { plugins: [pinia] } });
    await vi.waitFor(() => {
      expect(wrapper.findAll("button").some((button) => button.text() === "官方页")).toBe(
        deps.length > 0,
      );
    });
    return wrapper;
  }

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.resetAllMocks();
    vi.mocked(listRuntimeDependencies).mockResolvedValue([]);
    vi.mocked(listDownloads).mockResolvedValue({});
    vi.mocked(getDownloadsDir).mockResolvedValue("/data/downloads");
  });

  it("下载目录先展示占位文案、异步取回后替换", async () => {
    const wrapper = mount(DependenciesPanel, { global: { plugins: [pinia] } });
    // 探测是异步分支：挂载后立刻断言仍为占位。
    expect(wrapper.text()).toContain("探测中…");
    await vi.waitFor(() => expect(wrapper.text()).toContain("下载目录：/data/downloads"));
  });

  it("就绪依赖渲染徽标文案与必需标记，无下载按钮", async () => {
    const wrapper = await mountPanel([
      makeDep({
        id: "openfoam",
        name: "OpenFOAM",
        license: "GPL-3.0",
        strategy: "guided_install",
        required: true,
        download: null,
        ready: true,
      }),
    ]);
    expect(wrapper.text()).toContain("引导安装");
    expect(wrapper.text()).toContain("必需");
    expect(findButton(wrapper, "官方页")).toBeDefined();
    expect(wrapper.findAll("button").some((button) => button.text() === "下载")).toBe(false);
    const ready = wrapper.findAll("span").find((span) => span.text() === "就绪");
    expect(ready?.classes()).toContain("text-emerald-400");
  });

  it("未就绪必需依赖展示提示行，可选依赖展示 primary 下载按钮", async () => {
    const wrapper = await mountPanel([
      makeDep({ id: "openfoam", name: "OpenFOAM", required: true, ready: false, download: null }),
      makeDep(),
    ]);
    expect(wrapper.text()).toContain("请安装 Gmsh 后重新探测。");
    expect(wrapper.text()).toContain("未就绪");
    expect(wrapper.text()).toContain("可选");
    const notReady = wrapper.findAll("span").find((span) => span.text() === "未就绪");
    expect(notReady?.classes()).toContain("text-red-400");
    const downloadButton = findButton(wrapper, "下载");
    expect(downloadButton.attributes("title")).toBe("下载（GPL-2.0）");
    expect(downloadButton.classes().join(" ")).toContain("bg-emerald-500");
  });

  it("应用内副本就绪展示受管就绪文案", async () => {
    const wrapper = await mountPanel([makeDep({ ready: false, managedReady: true })]);
    expect(wrapper.text()).toContain("就绪（应用内副本）");
  });

  it.each([
    ["系统信息缺失回落 linux", null, "https://example.com/gmsh-linux.tgz"],
    ["macos 平台", "macos", "https://example.com/gmsh-macos.zip"],
    ["windows 平台", "windows", "https://example.com/gmsh-win.zip"],
  ])("下载地址按平台解析（%s）", async (_label, os, expectedUrl) => {
    useAppStore().info = os === null ? null : { name: "kairos", version: "0.1.0", os };
    vi.mocked(downloadComponentFile).mockResolvedValue(savedFixture);
    const wrapper = await mountPanel([makeDep()]);
    await findButton(wrapper, "下载").trigger("click");
    await vi.waitFor(() =>
      expect(downloadComponentFile).toHaveBeenCalledWith("gmsh", expectedUrl, expect.any(Function)),
    );
  });

  it("下载进度行内展示，成功后记录已下载与已保存", async () => {
    let release: ((saved: SavedDownload) => void) | undefined;
    vi.mocked(downloadComponentFile).mockImplementation(
      () =>
        new Promise<SavedDownload>((resolve) => {
          release = resolve;
        }),
    );
    const wrapper = await mountPanel([makeDep()]);
    await findButton(wrapper, "下载").trigger("click");
    const onProgress = vi.mocked(downloadComponentFile).mock.calls[0]![2]!;
    onProgress(55);
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("55%");
    expect(wrapper.find("[style]").attributes("style")).toContain("width: 55%");

    release!({ ...savedFixture, extractDir: "/downloads/gmsh" });
    await vi.waitFor(() => expect(wrapper.text()).toContain("已下载 gmsh.tgz（0.0 MB，已解压）"));
    expect(wrapper.text()).toContain("解压目录：/downloads/gmsh");
    expect(wrapper.text()).toContain("已解压：/downloads/gmsh");
    // 已下载过 → 「重新下载」走 ghost 描边变体（无实心主色类）。
    const again = findButton(wrapper, "重新下载");
    expect(again.classes()).not.toContain("bg-emerald-500");
  });

  it("下载失败落定失败阶段并展示原因", async () => {
    vi.mocked(downloadComponentFile).mockRejectedValue(new Error("网络中断"));
    const wrapper = await mountPanel([makeDep()]);
    await findButton(wrapper, "下载").trigger("click");
    await vi.waitFor(() => expect(wrapper.text()).toContain("下载失败：网络中断"));
  });

  it("跨会话已下载清单恢复：重新下载按钮与已下载文案", async () => {
    vi.mocked(listDownloads).mockResolvedValue({
      gmsh: { fileName: "gmsh.tgz", sizeBytes: 1024, downloadedAtMs: 0, extractDir: null },
    });
    const wrapper = await mountPanel([makeDep()]);
    await vi.waitFor(() => expect(wrapper.text()).toContain("已下载 gmsh.tgz（0.0 MB）"));
    expect(findButton(wrapper, "重新下载")).toBeDefined();
  });

  it("会话内保存记录展示已保存路径（未解压）", async () => {
    useDependenciesStore().savedDownloads = { gmsh: savedFixture };
    const wrapper = await mountPanel([makeDep()]);
    expect(wrapper.text()).toContain("已保存：/downloads/gmsh.tgz");
  });

  it("下载阶段清除后进度行消失", async () => {
    const wrapper = await mountPanel([makeDep()]);
    const deps = useDependenciesStore();
    deps.setStage("gmsh", { stage: "downloading", percent: 40 });
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("40%");
    deps.setStage("gmsh", null);
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).not.toContain("40%");
  });

  it("检查更新发现新版本：展示更新行、当前版本未知回落与更新按钮", async () => {
    vi.mocked(checkDependencyUpdate).mockResolvedValue({
      componentId: "gmsh",
      installedTag: null,
      latestTag: "v2.0",
      updateAvailable: true,
    });
    useDependenciesStore().savedDownloads = { gmsh: savedFixture };
    const wrapper = await mountPanel([makeDep({ updatable: true })]);
    await findButton(wrapper, "检查更新").trigger("click");
    await vi.waitFor(() => expect(wrapper.text()).toContain("发现新版本 v2.0（当前 未知）"));
    expect(findButton(wrapper, "更新到新版")).toBeDefined();
  });

  it("检查更新已是最新（版本已知）展示绿色提示", async () => {
    vi.mocked(checkDependencyUpdate).mockResolvedValue({
      componentId: "gmsh",
      installedTag: "v1.0",
      latestTag: "v1.0",
      updateAvailable: false,
    });
    const wrapper = await mountPanel([makeDep({ updatable: true })]);
    await useDependenciesStore().checkUpdate("gmsh");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("已是最新版本（v1.0）。");
  });

  it("检查更新已安装但版本未知展示灰色提示", async () => {
    vi.mocked(checkDependencyUpdate).mockResolvedValue({
      componentId: "gmsh",
      installedTag: null,
      latestTag: null,
      updateAvailable: false,
    });
    const wrapper = await mountPanel([makeDep({ updatable: true })]);
    await useDependenciesStore().checkUpdate("gmsh");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("已安装（版本标识未知，重新下载可获得版本标记）。");
  });

  it("检查更新失败设置全局错误且不展示更新提示", async () => {
    vi.mocked(checkDependencyUpdate).mockRejectedValue(new Error("离线"));
    const wrapper = await mountPanel([makeDep({ updatable: true })]);
    await useDependenciesStore().checkUpdate("gmsh");
    await wrapper.vm.$nextTick();
    expect(useAppStore().error?.message).toBe("离线");
    expect(wrapper.text()).not.toContain("已是最新版本");
  });

  it("moldingfoam 已下载出现部署到虚拟机按钮，其他组件不出现", async () => {
    const deps = useDependenciesStore();
    deps.savedDownloads = { gmsh: savedFixture };
    const wrapper = await mountPanel([makeDep()]);
    expect(wrapper.findAll("button").some((button) => button.text() === "部署到虚拟机")).toBe(
      false,
    );

    deps.savedDownloads = { moldingfoam: savedFixture };
    vi.mocked(deployVmBundle).mockResolvedValue("ok");
    vi.mocked(listRuntimeDependencies).mockResolvedValue([
      makeDep({ id: "moldingfoam", name: "MoldingFoam", download: null }),
    ]);
    const wrapper2 = mount(DependenciesPanel, { global: { plugins: [pinia] } });
    // store 仍持有上一次探测结果，须等第二次探测把 moldingfoam 行渲染出来。
    await vi.waitFor(() => {
      expect(wrapper2.findAll("button").some((button) => button.text() === "部署到虚拟机")).toBe(
        true,
      );
    });
    await findButton(wrapper2, "部署到虚拟机").trigger("click");
    await vi.waitFor(() => expect(deployVmBundle).toHaveBeenCalledTimes(1));
  });

  it("官方页按钮跳转，失败时设置全局错误", async () => {
    const wrapper = await mountPanel([makeDep()]);
    vi.mocked(openDependencyPage).mockResolvedValue(undefined);
    await findButton(wrapper, "官方页").trigger("click");
    await vi.waitFor(() => expect(openDependencyPage).toHaveBeenCalledWith("https://gmsh.sh"));

    vi.mocked(openDependencyPage).mockRejectedValue(new Error("无法打开"));
    await findButton(wrapper, "官方页").trigger("click");
    await vi.waitFor(() => expect(useAppStore().error?.message).toBe("无法打开"));
  });

  it("打开下载目录按钮调用系统文件管理器", async () => {
    vi.mocked(openDownloadsDir).mockResolvedValue("/data/downloads");
    const wrapper = await mountPanel([makeDep()]);
    await findButton(wrapper, "打开下载目录").trigger("click");
    await vi.waitFor(() => expect(openDownloadsDir).toHaveBeenCalledTimes(1));
  });

  it("探测失败设置全局错误", async () => {
    vi.mocked(listRuntimeDependencies).mockRejectedValue(new Error("探测失败"));
    mount(DependenciesPanel, { global: { plugins: [pinia] } });
    await vi.waitFor(() => expect(useAppStore().error?.message).toBe("探测失败"));
  });

  it("忙碌时打开目录与重新探测按钮禁用", async () => {
    const wrapper = await mountPanel([makeDep()]);
    useAppStore().busy = "正在求解…";
    await wrapper.vm.$nextTick();
    expect(findButton(wrapper, "打开下载目录").attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "重新探测").attributes("disabled")).toBeDefined();
  });

  it("未知策略与许可回落到许可文案与中性配色", async () => {
    const wrapper = await mountPanel([
      makeDep({
        strategy: "unknown_strategy" as InstallStrategy,
        licenseKind: "apache" as LicenseKind,
      }),
    ]);
    expect(wrapper.text()).toContain("GPL-2.0");
    const badge = wrapper.find(".rounded-full");
    expect(badge.classes().join(" ")).toContain("border-zinc-700");
  });
});
