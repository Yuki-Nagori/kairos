<script setup lang="ts">
/** 命令面板悬浮层：⌘K 唤起，过滤命令注册表，回车/点击执行高亮项。 */
import { useCommandPalette } from "./useCommandPalette";

const { open, query, activeIndex, filtered, closePalette, runCommand } = useCommandPalette();
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-50 flex items-start justify-center bg-black/40 pt-24"
      @click.self="closePalette()"
    >
      <div
        class="w-120 max-w-[90vw] overflow-hidden rounded-xl border border-zinc-700 bg-zinc-900 shadow-2xl"
      >
        <input
          v-model="query"
          type="text"
          autofocus
          placeholder="搜索命令…"
          class="w-full border-b border-zinc-800 bg-transparent px-4 py-3 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none"
        />
        <ul class="max-h-80 overflow-y-auto py-1">
          <li v-for="(command, index) in filtered" :key="command.id">
            <button
              type="button"
              class="flex w-full items-center gap-2 px-4 py-2 text-left text-xs"
              :class="
                index === activeIndex
                  ? 'bg-emerald-500/15 text-emerald-300'
                  : 'text-zinc-300 hover:bg-zinc-800/60'
              "
              @mouseenter="activeIndex = index"
              @click="runCommand(command.run)"
            >
              <span class="text-zinc-500">{{ command.group }}</span>
              <span>{{ command.label }}</span>
              <span v-if="command.shortcut" class="ml-auto font-mono text-[10px] text-zinc-500">{{
                command.shortcut
              }}</span>
            </button>
          </li>
          <li v-if="filtered.length === 0" class="px-4 py-3 text-xs text-zinc-500">
            没有匹配的命令
          </li>
        </ul>
      </div>
    </div>
  </Teleport>
</template>
