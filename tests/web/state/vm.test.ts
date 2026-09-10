import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  appStore,
  initialAppState,
  installVmAction,
  openVmShellAction,
  refreshVmStatus,
  sendVmShellLine,
  startVmAction,
  stopVmAction,
  stopVmShellAction,
} from "../../../src-web/state";
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
}));

const runningStatus: VmStatus = {
  provider: "multipass",
  toolInstalled: true,
  instanceName: "kairos",
  instanceState: "running",
  hint: "虚拟机运行中，可进入 Shell。",
};

function resetMocks(): void {
  vi.mocked(getVmStatus).mockReset();
  vi.mocked(installVm).mockReset();
  vi.mocked(startVm).mockReset();
  vi.mocked(vmShellStart).mockReset();
  vi.mocked(vmShellSend).mockReset();
  vi.mocked(vmShellStop).mockReset();
  vi.mocked(stopVm).mockReset();
}

describe("vm state slice", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
    resetMocks();
  });

  afterEach(() => {
    appStore.set(initialAppState);
    resetMocks();
  });

  it("refresh stores the probed status", async () => {
    vi.mocked(getVmStatus).mockResolvedValue(runningStatus);
    await refreshVmStatus();
    expect(appStore.get().vmStatus).toEqual(runningStatus);
  });

  it("install streams logs into the shell output and rescans status", async () => {
    vi.mocked(installVm).mockImplementation(async (onLog) => {
      onLog("brew 部署中");
      return "安装完成";
    });
    vi.mocked(getVmStatus).mockResolvedValue(runningStatus);

    await installVmAction();

    expect(appStore.get().vmShellLogs).toEqual(["brew 部署中"]);
    expect(appStore.get().vmStatus).toEqual(runningStatus);
    expect(appStore.get().vmBusy).toBeNull();
  });

  it("start failures surface as errors and clear busy", async () => {
    vi.mocked(startVm).mockRejectedValue(new Error("镜像下载失败"));

    await startVmAction();

    expect(appStore.get().error?.message).toBe("镜像下载失败");
    expect(appStore.get().vmBusy).toBeNull();
  });

  it("sends shell lines without manual echo (PTY echoes)", async () => {
    vi.mocked(vmShellSend).mockResolvedValue(undefined);
    vi.mocked(vmShellStart).mockResolvedValue(undefined);

    await openVmShellAction();
    await sendVmShellLine("blockMesh");

    // 回显由 PTY 提供，前端不重复记录
    expect(appStore.get().vmShellLogs).toEqual([]);
    expect(vmShellSend).toHaveBeenCalledWith("blockMesh");

    await stopVmShellAction();
    expect(vmShellStop).toHaveBeenCalled();
    expect(appStore.get().vmShellLogs.at(-1)).toBe("── Shell 会话已结束 ──");
  });

  it("stop stops the instance and rescans", async () => {
    vi.mocked(stopVm).mockResolvedValue("虚拟机已停止。");
    vi.mocked(getVmStatus).mockResolvedValue({
      ...runningStatus,
      instanceState: "stopped",
      hint: "虚拟机已创建但未运行。",
    });

    await stopVmAction();

    expect(appStore.get().vmStatus?.instanceState).toBe("stopped");
    expect(appStore.get().vmBusy).toBeNull();
  });
});
