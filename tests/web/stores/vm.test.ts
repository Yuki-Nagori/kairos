import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAppStore } from "../../../src-web/stores/app";
import { useVmStore } from "../../../src-web/stores/vm";
import {
  deployVmBundle,
  getVmStatus,
  installVm,
  startVm,
  stopVm,
  vmShellSend,
  vmShellStart,
  vmShellStop,
} from "../../../src-web/api/vm";
import type { VmStatus } from "../../../src-web/types";

vi.mock("../../../src-web/api/vm", () => ({
  getVmStatus: vi.fn(),
  getDeployedReleaseTag: vi.fn(async () => null),
  installVm: vi.fn(),
  startVm: vi.fn(),
  vmShellStart: vi.fn(),
  vmShellSend: vi.fn(),
  vmShellStop: vi.fn(),
  stopVm: vi.fn(),
  deployVmBundle: vi.fn(),
}));

const runningStatus: VmStatus = {
  provider: "multipass",
  toolInstalled: true,
  instanceName: "kairos",
  instanceState: "running",
  hint: "虚拟机运行中，可进入 Shell。",
};

describe("vm store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetAllMocks();
  });

  it("refresh stores the probed status", async () => {
    vi.mocked(getVmStatus).mockResolvedValue(runningStatus);
    const vm = useVmStore();
    await vm.refreshVmStatus();
    expect(vm.vmStatus).toEqual(runningStatus);
  });

  it("reports refresh failures into the app store", async () => {
    vi.mocked(getVmStatus).mockRejectedValue(new Error("探测失败"));

    const app = useAppStore();
    const vm = useVmStore();
    await vm.refreshVmStatus();

    expect(vm.vmStatus).toBeNull();
    expect(app.error?.message).toBe("探测失败");
  });

  it("install streams logs into the shell output and rescans status", async () => {
    vi.mocked(installVm).mockImplementation(async (onLog) => {
      onLog("brew 部署中");
      return "安装完成";
    });
    vi.mocked(getVmStatus).mockResolvedValue(runningStatus);

    const app = useAppStore();
    const vm = useVmStore();
    await vm.installVm();

    expect(vm.vmShellLogs).toEqual(["brew 部署中"]);
    expect(vm.vmStatus).toEqual(runningStatus);
    expect(vm.vmBusy).toBeNull();
    expect(app.error).toBeNull();
  });

  it("start streams logs and rescans status on success", async () => {
    vi.mocked(startVm).mockImplementation(async (onLog) => {
      onLog("下载镜像中");
      return "虚拟机已就绪";
    });
    vi.mocked(getVmStatus).mockResolvedValue(runningStatus);

    const app = useAppStore();
    const vm = useVmStore();
    await vm.startVm();

    expect(vm.vmShellLogs).toEqual(["下载镜像中"]);
    expect(vm.vmStatus).toEqual(runningStatus);
    expect(vm.vmBusy).toBeNull();
    expect(app.error).toBeNull();
  });

  it("start failures surface as errors and clear busy", async () => {
    vi.mocked(startVm).mockRejectedValue(new Error("镜像下载失败"));

    const app = useAppStore();
    const vm = useVmStore();
    await vm.startVm();

    expect(app.error?.message).toBe("镜像下载失败");
    expect(vm.vmBusy).toBeNull();
  });

  it("deploy streams bundle logs and rescans status on success", async () => {
    vi.mocked(deployVmBundle).mockImplementation(async (onLog) => {
      onLog("传输 bundle 中");
      return "部署完成";
    });
    vi.mocked(getVmStatus).mockResolvedValue(runningStatus);

    const app = useAppStore();
    const vm = useVmStore();
    const { getDeployedReleaseTag } = (await import("../../../src-web/api/vm")) as unknown as {
      getDeployedReleaseTag: ReturnType<typeof vi.fn>;
    };
    vi.mocked(getDeployedReleaseTag).mockResolvedValue("v0.2.0");
    await vm.deployVmBundle();

    expect(vm.vmShellLogs).toEqual(["传输 bundle 中"]);
    expect(vm.vmStatus).toEqual(runningStatus);
    expect(vm.vmBusy).toBeNull();
    expect(vm.deployedReleaseTag).toBe("v0.2.0");
    expect(app.error).toBeNull();
  });

  it("刷新已部署版本失败进全局错误", async () => {
    const { getDeployedReleaseTag } = (await import("../../../src-web/api/vm")) as unknown as {
      getDeployedReleaseTag: ReturnType<typeof vi.fn>;
    };
    vi.mocked(getDeployedReleaseTag).mockRejectedValueOnce(new Error("multipass 不可用"));
    const app = useAppStore();
    const vm = useVmStore();
    await vm.refreshDeployedReleaseTag();
    expect(app.error?.message).toBe("multipass 不可用");
    expect(vm.deployedReleaseTag).toBeNull();
  });

  it("deploy failures surface as errors and clear busy", async () => {
    vi.mocked(deployVmBundle).mockRejectedValue(new Error("传输中断"));

    const app = useAppStore();
    const vm = useVmStore();
    await vm.deployVmBundle();

    expect(app.error?.message).toBe("传输中断");
    expect(vm.vmBusy).toBeNull();
  });

  it("refuses a second action while one is in flight", async () => {
    const vm = useVmStore();
    vm.vmBusy = "install";

    await vm.installVm();

    expect(installVm).not.toHaveBeenCalled();
    expect(vm.vmBusy).toBe("install");
  });

  it("sends shell lines without manual echo (PTY echoes)", async () => {
    vi.mocked(vmShellSend).mockResolvedValue(undefined);
    vi.mocked(vmShellStart).mockImplementation(async (onLog) => {
      onLog("kairos-shell$ ");
      return undefined;
    });

    const vm = useVmStore();
    await vm.openShell();
    // Shell 会话的输出行持续进入终端面板
    expect(vm.vmShellLogs).toEqual(["kairos-shell$ "]);

    await vm.sendShellLine("blockMesh");
    // 输入回显由 PTY 提供，前端不重复记录
    expect(vm.vmShellLogs).toEqual(["kairos-shell$ "]);
    expect(vmShellSend).toHaveBeenCalledWith("blockMesh");

    await vm.stopShell();
    expect(vmShellStop).toHaveBeenCalled();
    expect(vm.vmShellLogs.at(-1)).toBe("── Shell 会话已结束 ──");
  });

  it("reports shell send failures", async () => {
    vi.mocked(vmShellSend).mockRejectedValue(new Error("会话未开启"));

    const app = useAppStore();
    const vm = useVmStore();
    await vm.sendShellLine("ls");

    expect(app.error?.message).toBe("会话未开启");
  });

  it("reports stopShell failures and skips the closing log", async () => {
    vi.mocked(vmShellStop).mockRejectedValue(new Error("停止失败"));

    const app = useAppStore();
    const vm = useVmStore();
    await vm.stopShell();

    expect(app.error?.message).toBe("停止失败");
    expect(vm.vmShellLogs).toEqual([]);
  });

  it("reports openShell failures via the busy wrapper", async () => {
    vi.mocked(vmShellStart).mockRejectedValue(new Error("无法启动 Shell"));

    const app = useAppStore();
    const vm = useVmStore();
    await vm.openShell();

    expect(app.error?.message).toBe("无法启动 Shell");
    expect(vm.vmBusy).toBeNull();
  });

  it("stop stops the instance and rescans", async () => {
    vi.mocked(stopVm).mockResolvedValue("虚拟机已停止。");
    vi.mocked(getVmStatus).mockResolvedValue({
      ...runningStatus,
      instanceState: "stopped",
      hint: "虚拟机已创建但未运行。",
    });

    const vm = useVmStore();
    await vm.stopVm();

    expect(vm.vmStatus?.instanceState).toBe("stopped");
    expect(vm.vmBusy).toBeNull();
  });

  it("showPanel opens and togglePanel flips the drawer", () => {
    const vm = useVmStore();
    expect(vm.vmPanelVisible).toBe(false);

    vm.showPanel();
    expect(vm.vmPanelVisible).toBe(true);

    vm.togglePanel();
    expect(vm.vmPanelVisible).toBe(false);

    vm.togglePanel();
    expect(vm.vmPanelVisible).toBe(true);
  });

  it("caps the shell log ring buffer at 500 lines", () => {
    const vm = useVmStore();
    for (let index = 0; index < 505; index += 1) {
      vm.appendShellLog(`line-${index}`);
    }
    expect(vm.vmShellLogs).toHaveLength(500);
    expect(vm.vmShellLogs[0]).toBe("line-5");
    expect(vm.vmShellLogs.at(-1)).toBe("line-504");
  });
});
