<script setup lang="ts">
/** 关于对话框：版本/平台与求解基座致谢，内容与原生「帮助 → 关于」一致。 */
import { useAppStore } from "../../stores/app";

defineProps<{ open: boolean }>();
defineEmits<{ close: [] }>();

const app = useAppStore();
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-60 flex items-center justify-center bg-black/50"
      @click.self="$emit('close')"
    >
      <div class="w-80 rounded-xl border border-zinc-700 bg-zinc-900 p-5 text-center shadow-2xl">
        <img src="/icon.svg" alt="Kairos" class="mx-auto size-10" />
        <h2 class="mt-2 text-sm font-semibold text-zinc-100">Kairos</h2>
        <p class="mt-1 text-xs text-zinc-500">
          {{ app.info ? `v${app.info.version} · ${app.info.os}` : "注塑成型 CAE 仿真工作台" }}
        </p>
        <p class="mt-3 text-[11px] leading-5 text-zinc-500">感谢 OpenFOAM 提供求解基座</p>
        <button
          type="button"
          class="mt-4 rounded-md border border-zinc-700 px-3 py-1 text-xs text-zinc-300 transition-colors hover:border-emerald-500/60 hover:text-emerald-300"
          @click="$emit('close')"
        >
          关闭
        </button>
      </div>
    </div>
  </Teleport>
</template>
