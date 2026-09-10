<script setup lang="ts">
/** 应用菜单栏：品牌 + 一级菜单 + 命令搜索入口（⌘K）。交互逻辑见 useMenuBar。 */
import AboutDialog from "./AboutDialog.vue";
import { useMenuBar } from "./useMenuBar";
import { useCommandPalette } from "../command-palette/useCommandPalette";

const { menuGroups, openIndex, aboutOpen, toggleMenu, hoverMenu, runCommand } = useMenuBar();
const { openPalette } = useCommandPalette();
</script>

<template>
  <header
    data-menu-root
    class="flex h-9 shrink-0 items-center gap-1 border-b border-zinc-800 bg-zinc-900 px-3 select-none"
  >
    <img src="/icon.svg" alt="Kairos" class="mr-1.5 size-4 shrink-0" />
    <h1 class="mr-3 text-xs font-semibold text-emerald-400">Kairos</h1>
    <nav class="flex items-center">
      <div v-for="(group, index) in menuGroups" :key="group.label" class="relative">
        <button
          type="button"
          class="rounded-md px-2.5 py-1 text-xs transition-colors"
          :class="
            openIndex === index
              ? 'bg-zinc-800 text-zinc-100'
              : 'text-zinc-400 hover:bg-zinc-800/60 hover:text-zinc-200'
          "
          @click="toggleMenu(index)"
          @mouseenter="hoverMenu(index)"
        >
          {{ group.label }}
        </button>
        <div
          v-show="openIndex === index"
          class="absolute top-full left-0 z-50 mt-1 min-w-44 rounded-lg border border-zinc-700 bg-zinc-900 py-1 shadow-xl"
        >
          <button
            v-for="command in group.commands"
            :key="command.id"
            type="button"
            class="flex w-full items-center gap-4 px-3 py-1.5 text-left text-xs whitespace-nowrap text-zinc-300 hover:bg-zinc-800 hover:text-zinc-100"
            @click="runCommand(command)"
          >
            <span>{{ command.label }}</span>
            <span v-if="command.shortcut" class="ml-auto font-mono text-[10px] text-zinc-500">{{
              command.shortcut
            }}</span>
          </button>
        </div>
      </div>
    </nav>
    <span class="flex-1" />
    <button
      type="button"
      class="w-52 rounded-md border border-zinc-700 bg-zinc-800/60 px-2.5 py-1 text-left text-xs text-zinc-500 transition-colors hover:border-emerald-500/60 hover:text-zinc-300"
      title="搜索命令（⌘K）"
      @click="openPalette()"
    >
      搜索命令…<span class="float-right font-mono text-[10px]">⌘K</span>
    </button>
    <AboutDialog :open="aboutOpen" @close="aboutOpen = false" />
  </header>
</template>
