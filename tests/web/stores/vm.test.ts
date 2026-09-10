import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAppStore } from "../../../src-web/stores/app";
import { useVmStore } from "../../../src-web/stores/vm";
import {
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

  it("install streams logs into the shell output and rescans status", async () => {
    vi.mocked(installVm).mockImplementation(async (onLog) => {
      onLog("brew 部署中");
      return "安装完成";
    });
    vi.mocked(getVmStatus).mockResolvedValue(runningStatus);

    const vm = useVmStore();
    await vm.installVm();

    expect(vm.vmShellLogs).toEqual(["brew 部署中"]);
    expect(vm.vmStatus).toEqual(runningStatus);
    expect(vm.vmBusy).toBeNull();
  });

  it("start failures surface as errors and clear busy", async () => {
    vi.mocked(startVm).mockRejectedValue(new Error("镜像下载失败"));

    const app = useAppStore();
    const vm = useVmStore();
    await vm.startVm();

    expect(app.error?.message).toBe("镜像下载失败");
    expect(vm.vmBusy).toBeNull();
  });

  it("sends shell lines without manual echo (PTY echoes)", async () => {
    vi.mocked(vmShellSend).mockResolvedValue(undefined);
    vi.mocked(vmShellStart).mockResolvedValue(undefined);

    const vm = useVmStore();
    await vm.openShell();
    await vm.sendShellLine("blockMesh");

    // 回显由 PTY 提供，前端不重复记录
    expect(vm.vmShellLogs).toEqual([]);
    expect(vmShellSend).toHaveBeenCalledWith("blockMesh");

    await vm.stopShell();
    expect(vmShellStop).toHaveBeenCalled();
    expect(vm.vmShellLogs.at(-1)).toBe("── Shell 会话已结束 ──");
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
