<script setup lang="ts">
/**
 * 关于对话框：Windows / Linux 与浏览器预览的关于入口（macOS 走系统面板）。
 * 顶格布局，不保留图标区；版本/平台取自 system info。
 */
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
      <div class="w-80 rounded-xl border border-zinc-700 bg-zinc-900 p-5 shadow-2xl">
        <div class="flex items-baseline justify-between">
          <h2 class="text-sm font-semibold text-zinc-100">Kairos</h2>
          <span class="font-mono text-[11px] text-zinc-500">
            {{ app.info ? `v${app.info.version} · ${app.info.os}` : "注塑成型 CAE 仿真工作台" }}
          </span>
        </div>
        <p class="mt-3 text-[11px] leading-5 text-zinc-500">
          注塑成型 CAE 仿真工作台。感谢 OpenFOAM (openfoam.org) 提供求解基座，感谢 moldingFoam
          项目提供注塑求解模块。
        </p>
        <p class="mt-2 text-[11px] text-zinc-600">Copyright © 2026 Yuki</p>
        <div class="mt-4 flex justify-end">
          <button
            type="button"
            class="rounded-md border border-zinc-700 px-3 py-1 text-xs text-zinc-300 transition-colors hover:border-emerald-500/60 hover:text-emerald-300"
            @click="$emit('close')"
          >
            关闭
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
