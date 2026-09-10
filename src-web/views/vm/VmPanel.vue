<script setup lang="ts">
/** 虚拟机面板：逻辑见 useVmPanel。 */
import { useVmPanel } from "./useVmPanel";
import UiButton from "../../components/ui/UiButton.vue";
import Card from "../../components/ui/UiCard.vue";
import ShellIcon from "../../components/ui/icons/ShellIcon.vue";

// 模板 ref（outputRef/promptInput）在 composable 内按名绑定，此处无需解构。
const {
  vm,
  busy,
  statusHint,
  terminalTitle,
  native,
  toolInstalled,
  instanceState,
  outputText,
  onRefresh,
  command,
  focusPrompt,
  send,
} = useVmPanel();
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
