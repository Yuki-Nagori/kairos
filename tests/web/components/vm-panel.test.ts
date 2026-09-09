import { beforeEach, describe, expect, it, vi } from "vitest";
import { createVmPanel } from "../../../src-web/components/panels/vm-panel";
import { appStore, initialAppState } from "../../../src-web/state";
import { getVmStatus, vmShellSend } from "../../../src-web/services/vm";
import type { VmProvider, VmState, VmStatus } from "../../../src-web/types";

vi.mock("../../../src-web/services/vm", () => ({
  getVmStatus: vi.fn(),
  installVm: vi.fn(),
  startVm: vi.fn(),
  vmShellStart: vi.fn(),
  vmShellSend: vi.fn(),
  vmShellStop: vi.fn(),
  stopVm: vi.fn(),
}));

const runningStatus: VmStatus = {
  provider: "multipass" satisfies VmProvider,
  toolInstalled: true,
  instanceName: "kairos",
  instanceState: "running" satisfies VmState,
  hint: "虚拟机运行中，可进入 Shell。",
};

function findButton(root: HTMLElement, label: string): HTMLButtonElement {
  const found = [...root.querySelectorAll("button")].find(
    (candidate) => candidate.textContent === label,
  );
  if (found === undefined) {
    throw new Error(`找不到按钮：${label}`);
  }
  return found;
}

describe("vm panel", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
    vi.mocked(getVmStatus).mockReset();
    vi.mocked(vmShellSend).mockReset();
    vi.mocked(getVmStatus).mockResolvedValue(runningStatus);
  });

  it("renders hint and disables shell actions before probing", () => {
    const root = createVmPanel();
    expect(root.textContent).toContain("尚未探测");
    expect(findButton(root, "安装虚拟机").disabled).toBe(false);
    expect(findButton(root, "进入 Shell").disabled).toBe(true);
    expect(findButton(root, "关闭虚拟机").disabled).toBe(true);
  });

  it("enables shell actions and renders logs when running", () => {
    appStore.set({ vmStatus: runningStatus, vmShellLogs: ["── Shell 会话已建立 ──"] });
    const root = createVmPanel();
    expect(root.textContent).toContain("虚拟机运行中");
    expect(findButton(root, "安装虚拟机").disabled).toBe(true);
    expect(findButton(root, "进入 Shell").disabled).toBe(false);
    expect(findButton(root, "关闭虚拟机").disabled).toBe(false);
    expect(root.querySelector("pre")?.textContent).toContain("Shell 会话已建立");
    // 模拟终端：shell.svg 图标（currentColor）出现在面板 logo 与终端标题栏。
    expect(root.querySelectorAll("svg").length).toBeGreaterThanOrEqual(2);
    expect(root.textContent).toContain("shell · kairos");
  });

  it("short-circuits vm buttons on native Linux", () => {
    appStore.set({
      vmStatus: {
        provider: "native",
        toolInstalled: true,
        instanceName: "localhost",
        instanceState: "running",
        hint: "Linux 原生环境，无需虚拟机，可直接进入 Shell。",
      },
    });
    const root = createVmPanel();
    expect(root.textContent).toContain("无需虚拟机");
    expect(findButton(root, "安装虚拟机").disabled).toBe(true);
    expect(findButton(root, "启动虚拟机").disabled).toBe(true);
    expect(findButton(root, "关闭虚拟机").disabled).toBe(true);
    expect(findButton(root, "进入 Shell").disabled).toBe(false);
  });

  it("sends trimmed input on Enter and clears the field", async () => {
    vi.mocked(vmShellSend).mockResolvedValue(undefined);
    const root = createVmPanel();
    const input = root.querySelector("input") as HTMLInputElement;

    input.value = "  ls -la  ";
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
    await vi.waitFor(() => expect(vmShellSend).toHaveBeenCalledWith("ls -la"));
    expect(input.value).toBe("");
  });
});
