<script setup lang="ts">
/**
 * 应用菜单栏：品牌行 + 命令搜索入口（⌘K）。
 * 菜单本体在系统菜单栏（macOS 顶部系统栏，Windows/Linux 在窗口标题栏下方，
 * 由 Tauri 原生菜单渲染，动作经 menu-action 事件桥回 web），窗口内不再重复。
 */
import { commandPaletteShortcutLabel } from "../../utils/environment";
import { useCommandPalette } from "../command-palette/useCommandPalette";

const { openPalette } = useCommandPalette();
const shortcutLabel = commandPaletteShortcutLabel();
</script>

<template>
  <header
    class="flex h-9 shrink-0 items-center gap-1 border-b border-zinc-800 bg-zinc-900 px-3 select-none"
  >
    <img src="/icon.svg" alt="Kairos" class="mr-1.5 size-4 shrink-0" />
    <h1 class="text-xs font-semibold text-emerald-400">Kairos</h1>
    <span class="text-xs text-zinc-600">CAE 仿真</span>
    <span class="flex-1" />
    <button
      type="button"
      class="w-52 rounded-md border border-zinc-700 bg-zinc-800/60 px-2.5 py-1 text-left text-xs text-zinc-500 transition-colors hover:border-emerald-500/60 hover:text-zinc-300"
      :title="`搜索命令（${shortcutLabel}）`"
      @click="openPalette()"
    >
      搜索命令…<span class="float-right font-mono text-[10px]">{{ shortcutLabel }}</span>
    </button>
  </header>
</template>
