<script setup lang="ts">
/**
 * 应用标题栏（菜单栏行）：三端统一为 Overlay 标题栏
 * （tauri-plugin-decoration 运行时管理装饰），本行即标题栏——
 * 拖拽移动、品牌、菜单、命令搜索入口都在这一行。
 * 菜单本体：macOS 在系统菜单栏；Windows/Linux 无边框由本行 web 菜单承载。
 * 窗口控制：macOS 为系统红绿灯，Windows/Linux 为插件内嵌控制按钮
 * （Windows 11 含 Snap Layout 热区）——行首行尾按插件发布的 clearance
 * 变量避让，不硬编码按钮宽度。
 */
import { useShell } from "../../composables/useShell";
import AboutDialog from "./AboutDialog.vue";
import { useAboutDialog } from "./useAboutDialog";
import { useMenuBar } from "./useMenuBar";
import { useCommandPalette } from "../command-palette/useCommandPalette";

const shell = useShell();
const { menuGroups, openIndex, toggleMenu, hoverMenu, runCommand } = useMenuBar();
const { aboutOpen, hideAbout } = useAboutDialog();
const { openPalette } = useCommandPalette();
</script>

<template>
  <header
    data-menu-root
    data-tauri-drag-region
    class="flex h-9 shrink-0 items-center gap-1 border-b border-zinc-800 bg-zinc-900 select-none"
    :style="{
      paddingLeft: 'max(12px, var(--tauri-plugin-decoration-left-clearance, 0px))',
      paddingRight: 'max(12px, var(--tauri-plugin-decoration-right-clearance, 0px))',
    }"
  >
    <img src="/icon.svg" alt="Kairos" data-tauri-drag-region class="mr-1.5 size-4 shrink-0" />
    <h1 data-tauri-drag-region class="text-xs font-semibold text-emerald-400">Kairos</h1>
    <span data-tauri-drag-region class="text-xs text-zinc-600">CAE 仿真</span>

    <!-- 系统菜单栏不可用时（Windows/Linux 无边框 + 浏览器预览）：标题栏内下拉菜单 -->
    <nav v-if="!shell.nativeMenu" class="ml-3 flex items-center">
      <div v-for="(group, index) in menuGroups" :key="group.label" class="relative">
        <button
          type="button"
          class="rounded-md px-2.5 py-1 text-xs transition-colors"
          :class="
            openIndex === index
              ? 'bg-zinc-800 text-zinc-100'
              : 'text-zinc-400 hover:bg-zinc-800/60 hover:text-zinc-200'
          "
          aria-haspopup="menu"
          :aria-expanded="openIndex === index"
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

    <span data-tauri-drag-region class="flex-1 self-stretch" />
    <button
      type="button"
      class="w-52 rounded-md border border-zinc-700 bg-zinc-800/60 px-2.5 py-1 text-left text-xs text-zinc-500 transition-colors hover:border-emerald-500/60 hover:text-zinc-300"
      :title="`搜索命令（${shell.paletteShortcutLabel}）`"
      @click="openPalette()"
    >
      搜索命令…<span class="float-right font-mono text-[10px]">{{
        shell.paletteShortcutLabel
      }}</span>
    </button>

    <AboutDialog :open="aboutOpen" @close="hideAbout()" />
  </header>
</template>
