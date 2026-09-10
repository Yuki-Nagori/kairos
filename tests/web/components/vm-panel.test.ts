import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { defineComponent, h, nextTick } from "vue";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import VmPanel from "../../../src-web/views/vm/VmPanel.vue";
import { useVmPanel } from "../../../src-web/views/vm/useVmPanel";
import { useVmStore } from "../../../src-web/stores/vm";
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
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.mocked(getVmStatus).mockReset();
    vi.mocked(vmShellSend).mockReset();
    vi.mocked(getVmStatus).mockResolvedValue(runningStatus);
  });

  it("未探测时显示提示，Shell 动作不可用", () => {
    const wrapper = mount(VmPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("尚未探测");
    expect(findButton(wrapper, "安装虚拟机").attributes("disabled")).toBeUndefined();
    expect(findButton(wrapper, "进入 Shell").attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "关闭虚拟机").attributes("disabled")).toBeDefined();
  });

  it("运行中启用 Shell 动作并渲染终端日志", () => {
    const vm = useVmStore();
    vm.vmStatus = runningStatus;
    vm.vmShellLogs = ["── Shell 会话已建立 ──"];
    const wrapper = mount(VmPanel, { global: { plugins: [pinia] } });
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
    const vm = useVmStore();
    vm.vmStatus = {
      provider: "native",
      toolInstalled: true,
      instanceName: "localhost",
      instanceState: "running",
      hint: "Linux 原生环境，无需虚拟机，可直接进入 Shell。",
    };
    const wrapper = mount(VmPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("无需虚拟机");
    expect(findButton(wrapper, "安装虚拟机").attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "启动虚拟机").attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "关闭虚拟机").attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "进入 Shell").attributes("disabled")).toBeUndefined();
  });

  it("回车发送去空白命令并清空输入框", async () => {
    vi.mocked(vmShellSend).mockResolvedValue(undefined);
    const wrapper = mount(VmPanel, { global: { plugins: [pinia] } });
    const input = wrapper.find("input");

    await input.setValue("  ls -la  ");
    await input.trigger("keydown.enter");
    await vi.waitFor(() => expect(vmShellSend).toHaveBeenCalledWith("ls -la"));
    expect((input.element as HTMLInputElement).value).toBe("");
  });

  it("空白命令回车不发送", async () => {
    const wrapper = mount(VmPanel, { global: { plugins: [pinia] } });
    const input = wrapper.find("input");
    await input.setValue("   ");
    await input.trigger("keydown.enter");
    await input.setValue("");
    await input.trigger("keydown.enter");
    expect(vmShellSend).not.toHaveBeenCalled();
  });

  it("重新探测按钮触发状态刷新，忙碌时短路", async () => {
    const wrapper = mount(VmPanel, { global: { plugins: [pinia] } });
    await vi.waitFor(() => expect(getVmStatus).toHaveBeenCalledTimes(1));
    await findButton(wrapper, "重新探测").trigger("click");
    await vi.waitFor(() => expect(getVmStatus).toHaveBeenCalledTimes(2));

    const vm = useVmStore();
    vm.vmBusy = "install";
    await wrapper.vm.$nextTick();
    await findButton(wrapper, "重新探测").trigger("click");
    expect(getVmStatus).toHaveBeenCalledTimes(2);
    vm.vmBusy = null;
  });

  it("Shell 日志追加后把输出区滚动到底部", async () => {
    const vm = useVmStore();
    const wrapper = mount(VmPanel, { global: { plugins: [pinia] } });
    const output = wrapper.find("pre").element as HTMLPreElement;
    output.scrollTop = 0;
    vm.appendShellLog("第一行");
    vm.appendShellLog("第二行");
    await vi.waitFor(() => expect(output.scrollTop).toBe(output.scrollHeight));
  });

  it("点击终端区域聚焦提示符输入框", async () => {
    const focusSpy = vi.spyOn(HTMLInputElement.prototype, "focus");
    const wrapper = mount(VmPanel, { global: { plugins: [pinia] } });
    await wrapper.find("pre").trigger("click");
    expect(focusSpy).toHaveBeenCalledTimes(1);
  });

  it("工具已装但实例缺失：instanceState 保持 missing，启动可用而 Shell 入口禁用", () => {
    const vm = useVmStore();
    vm.vmStatus = {
      provider: "multipass" satisfies VmProvider,
      toolInstalled: true,
      instanceName: "kairos",
      instanceState: "missing" satisfies VmState,
      hint: "工具已安装，实例尚未创建。",
    };
    const wrapper = mount(VmPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("工具已安装，实例尚未创建。");
    expect(findButton(wrapper, "启动虚拟机").attributes("disabled")).toBeUndefined();
    expect(findButton(wrapper, "进入 Shell").attributes("disabled")).toBeDefined();
  });
});

describe("useVmPanel 防御分支（无输出区环境）", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.mocked(getVmStatus).mockReset();
    vi.mocked(getVmStatus).mockResolvedValue(runningStatus);
  });

  /** 无输出区、无按钮模板：直接读 composable 暴露的 computed。 */
  function mountPanellessHarness() {
    let panel!: ReturnType<typeof useVmPanel>;
    const wrapper = mount(
      defineComponent({
        setup() {
          panel = useVmPanel();
          return () => h("div");
        },
      }),
      { global: { plugins: [pinia] } },
    );
    return { wrapper, panel };
  }

  it("输出区引用缺失时滚动 watch 安全跳过", async () => {
    const { wrapper } = mountPanellessHarness();
    // 无 <pre ref="outputRef">：日志追加仍触发 watch，滚动逻辑短路。
    useVmStore().appendShellLog("一行日志");
    await nextTick();
    await nextTick();
    wrapper.unmount();
  });

  it("未探测时直读 computed：状态提示与实例状态走回落分支", () => {
    const { wrapper, panel } = mountPanellessHarness();
    expect(panel.statusHint.value).toContain("尚未探测");
    expect(panel.toolInstalled.value).toBe(false);
    // 模板里该分支被 !toolInstalled 短路遮蔽，须直读才能覆盖 ?? "missing"。
    expect(panel.instanceState.value).toBe("missing");
    expect(panel.terminalTitle.value).toBe("shell");
    wrapper.unmount();
  });

  it("探测后直读 computed：取真实状态而非回落值", () => {
    const { wrapper, panel } = mountPanellessHarness();
    useVmStore().vmStatus = runningStatus;
    expect(panel.statusHint.value).toBe(runningStatus.hint);
    expect(panel.instanceState.value).toBe("running");
    expect(panel.terminalTitle.value).toBe("shell · kairos");
    wrapper.unmount();
  });
});
