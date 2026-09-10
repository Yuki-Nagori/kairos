import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import VmPanel from "../../../src-web/views/vm/VmPanel.vue";
import { appStore, initialAppState } from "../../../src-web/state";
import { getVmStatus, vmShellSend } from "../../../src-web/api/vm";
import type { VmProvider, VmState, VmStatus } from "../../../src-web/types";

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
  provider: "multipass" satisfies VmProvider,
  toolInstalled: true,
  instanceName: "kairos",
  instanceState: "running" satisfies VmState,
  hint: "虚拟机运行中，可进入 Shell。",
};

function findButton(wrapper: ReturnType<typeof mount>, label: string) {
  const found = wrapper.findAll("button").find((candidate) => candidate.text() === label);
  if (found === undefined) {
    throw new Error(`找不到按钮：${label}`);
  }
  return found;
}

describe("VmPanel", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
    vi.mocked(getVmStatus).mockReset();
    vi.mocked(vmShellSend).mockReset();
    vi.mocked(getVmStatus).mockResolvedValue(runningStatus);
  });

  it("未探测时显示提示，Shell 动作不可用", () => {
    const wrapper = mount(VmPanel);
    expect(wrapper.text()).toContain("尚未探测");
    expect(findButton(wrapper, "安装虚拟机").attributes("disabled")).toBeUndefined();
    expect(findButton(wrapper, "进入 Shell").attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "关闭虚拟机").attributes("disabled")).toBeDefined();
  });

  it("运行中启用 Shell 动作并渲染终端日志", () => {
    appStore.set({ vmStatus: runningStatus, vmShellLogs: ["── Shell 会话已建立 ──"] });
    const wrapper = mount(VmPanel);
    expect(wrapper.text()).toContain("虚拟机运行中");
    expect(findButton(wrapper, "安装虚拟机").attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "进入 Shell").attributes("disabled")).toBeUndefined();
    expect(findButton(wrapper, "关闭虚拟机").attributes("disabled")).toBeUndefined();
    expect(wrapper.find("pre").text()).toContain("Shell 会话已建立");
    // 模拟终端：内联 SVG 图标出现在面板 logo 与终端标题栏。
    expect(wrapper.findAll("svg").length).toBeGreaterThanOrEqual(2);
    expect(wrapper.text()).toContain("shell · kairos");
  });

  it("Linux 原生环境短路虚拟机按钮", () => {
    appStore.set({
      vmStatus: {
        provider: "native",
        toolInstalled: true,
        instanceName: "localhost",
        instanceState: "running",
        hint: "Linux 原生环境，无需虚拟机，可直接进入 Shell。",
      },
    });
    const wrapper = mount(VmPanel);
    expect(wrapper.text()).toContain("无需虚拟机");
    expect(findButton(wrapper, "安装虚拟机").attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "启动虚拟机").attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "关闭虚拟机").attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "进入 Shell").attributes("disabled")).toBeUndefined();
  });

  it("回车发送去空白命令并清空输入框", async () => {
    vi.mocked(vmShellSend).mockResolvedValue(undefined);
    const wrapper = mount(VmPanel);
    const input = wrapper.find("input");

    await input.setValue("  ls -la  ");
    await input.trigger("keydown.enter");
    await vi.waitFor(() => expect(vmShellSend).toHaveBeenCalledWith("ls -la"));
    expect((input.element as HTMLInputElement).value).toBe("");
  });
});
