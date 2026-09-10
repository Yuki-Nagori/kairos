/**
 * 虚拟机面板（浮于工作区右下角）：Multipass（macOS）/ WSL2（Windows）的
 * 一键安装、启动、应用内 Shell 与关闭。全部逻辑在 vm store 与 Rust 适配层，
 * 这里只渲染；Shell 区模拟终端外观（图标标题栏 + 提示符行内输入）。
 */
import { computed, nextTick, onMounted, ref, useTemplateRef, watch } from "vue";
import { useVmStore } from "../../stores/vm";

export function useVmPanel() {
  const vm = useVmStore();

  const busy = computed(() => vm.vmBusy !== null);
  const statusHint = computed(
    () => vm.vmStatus?.hint ?? "尚未探测。点击「重新探测」检查虚拟机运行时。",
  );
  const terminalTitle = computed(() =>
    vm.vmStatus === null ? "shell" : `shell · ${vm.vmStatus.instanceName}`,
  );
  // Linux 原生环境无虚拟机语义：安装/启动/关闭一律短路。
  const native = computed(() => vm.vmStatus?.provider === "native");
  const toolInstalled = computed(() => vm.vmStatus?.toolInstalled ?? false);
  const instanceState = computed(() => vm.vmStatus?.instanceState ?? "missing");

  const outputText = computed(() => vm.vmShellLogs.join("\n"));

  // Card 的刷新按钮不外联 disabled，改在函数内守卫 busy。
  function onRefresh(): void {
    if (busy.value) {
      return;
    }
    void vm.refreshVmStatus();
  }

  // 模板 ref 用 useTemplateRef 按名绑定（模板 ref="同名" 不变）；
  // 逻辑住 composable 后 .vue 无需再持有同名 setup 绑定。
  const outputRef = useTemplateRef<HTMLPreElement>("outputRef");
  const promptInput = useTemplateRef<HTMLInputElement>("promptInput");
  const command = ref("");

  // store 原地 push 追加日志（数组引用不变），监听长度才能感知追加：
  // 等 DOM 更新完把输出区滚到底部。
  watch(
    () => vm.vmShellLogs.length,
    async () => {
      await nextTick();
      const output = outputRef.value;
      if (output) {
        output.scrollTop = output.scrollHeight;
      }
    },
  );

  function focusPrompt(): void {
    promptInput.value?.focus();
  }

  // 空行不发送；PTY 会回显输入行，前端无需补记日志。
  function send(): void {
    const line = command.value.trim();
    if (line.length === 0) {
      return;
    }
    command.value = "";
    void vm.sendShellLine(line);
  }

  onMounted(() => {
    void vm.refreshVmStatus();
  });

  return {
    vm,
    busy,
    statusHint,
    terminalTitle,
    native,
    toolInstalled,
    instanceState,
    outputText,
    onRefresh,
    // 模板 ref 随 composable 暴露；.vue 不必解构（经 refs 注册表按名同步）。
    outputRef,
    promptInput,
    command,
    focusPrompt,
    send,
  };
}
