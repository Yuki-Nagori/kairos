<script setup lang="ts">
/**
 * 虚拟机面板（浮于工作区右下角）：Multipass（macOS）/ WSL2（Windows）的
 * 一键安装、启动、应用内 Shell 与关闭。全部逻辑在 vm store 与 Rust 适配层，
 * 这里只渲染；Shell 区模拟终端外观（图标标题栏 + 提示符行内输入）。
 */
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useVmStore } from "../../stores/vm";
import UiButton from "../../components/ui/UiButton.vue";
import Card from "../../components/ui/UiCard.vue";
import ShellIcon from "../../components/ui/ShellIcon.vue";

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

// 原刷新按钮在动作进行中禁用——Card 的刷新按钮不外联 disabled，以守卫等价。
function onRefresh(): void {
  if (busy.value) {
    return;
  }
  void vm.refreshVmStatus();
}

// 模拟终端：深色窗口 + 输出滚动区 + 提示符行内输入。
const outputRef = ref<HTMLPreElement | null>(null);
const promptInput = ref<HTMLInputElement | null>(null);
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

// 点终端任意处聚焦输入行。
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
</script>

<template>
  <Card title="虚拟机" :status-hint="statusHint" refresh-label="重新探测" @refresh="onRefresh">
    <template #icon>
      <ShellIcon class="h-4 w-4 text-emerald-400" />
    </template>
    <div class="flex flex-wrap items-center gap-2">
      <UiButton :disabled="busy || native || toolInstalled" @click="vm.installVm()">
        安装虚拟机
      </UiButton>
      <UiButton
        :disabled="busy || native || !toolInstalled || instanceState === 'running'"
        @click="vm.startVm()"
      >
        启动虚拟机
      </UiButton>
      <UiButton
        :disabled="busy || !toolInstalled || instanceState === 'missing'"
        @click="vm.openShell()"
      >
        进入 Shell
      </UiButton>
      <UiButton variant="ghost" :disabled="busy" @click="vm.stopShell()"> 停止 Shell </UiButton>
      <UiButton
        variant="ghost"
        :disabled="busy || native || !toolInstalled || instanceState === 'missing'"
        @click="vm.stopVm()"
      >
        关闭虚拟机
      </UiButton>
    </div>
    <div
      class="overflow-hidden rounded-lg border border-zinc-800 bg-black font-mono select-none"
      @click="focusPrompt"
    >
      <div
        class="flex items-center gap-2 border-b border-zinc-800 bg-zinc-900 px-3 py-1.5 text-[10px] text-zinc-400"
      >
        <ShellIcon class="h-3.5 w-3.5 text-emerald-400" />
        <span>{{ terminalTitle }}</span>
      </div>
      <pre
        ref="outputRef"
        class="h-44 overflow-y-auto whitespace-pre-wrap break-all px-3 py-2 text-[11px] leading-4 text-emerald-300/90"
        >{{ outputText }}</pre>
      <div class="flex items-center gap-1.5 px-3 pb-2 text-[11px]">
        <span class="text-amber-400">❯</span>
        <input
          v-model="command"
          class="min-w-0 flex-1 border-none bg-transparent font-mono text-[11px] text-emerald-200 outline-none placeholder:text-zinc-600"
          placeholder="输入命令，回车发送…"
          autocomplete="off"
          spellcheck="false"
          @keydown.enter="send"
        />
      </div>
    </div>
  </Card>
</template>
