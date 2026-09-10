<script setup lang="ts">
/** 运行时依赖面板：逻辑见 useDependenciesPanel。 */
import { useDependenciesPanel } from "./useDependenciesPanel";
import Card from "../../components/ui/UiCard.vue";
import ShellIcon from "../../components/ui/icons/ShellIcon.vue";
import UiButton from "../../components/ui/UiButton.vue";

const { app, vm, deps, rows, downloadsDir, openDownloadsDir } = useDependenciesPanel();
</script>

<template>
  <Card title="运行时依赖" status-hint="尚未探测。点击「重新探测」检查 OpenFOAM 环境。">
    <template #icon>
      <ShellIcon class="h-4 w-4 text-emerald-400" />
    </template>
    <UiButton :disabled="app.busy !== null" @click="openDownloadsDir()">打开下载目录</UiButton>
    <p class="text-xs text-zinc-500">下载目录：{{ downloadsDir }}</p>
    <UiButton :disabled="app.busy !== null" @click="deps.refreshDependencies()">重新探测</UiButton>
    <div class="space-y-2">
      <template v-for="row in rows" :key="row.dep.id">
        <div
          class="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg border border-zinc-800 px-3 py-2 text-xs"
        >
          <UiButton v-if="row.dep.id === 'moldingfoam' && row.already" @click="vm.deployVmBundle()">
            部署到虚拟机
          </UiButton>
          <UiButton v-if="row.dep.updatable && row.already" @click="deps.checkUpdate(row.dep.id)">
            检查更新
          </UiButton>
          <span class="font-semibold text-zinc-200">{{ row.dep.name }}</span>
          <span
            class="inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[10px]"
            :class="row.badgeClass"
          >
            {{ row.badgeText }}
          </span>
          <span :class="row.readyClass">{{ row.readyText }}</span>
          <span class="text-zinc-600">{{ row.dep.required ? "必需" : "可选" }}</span>
          <UiButton @click="deps.openDependencyPage(row.dep.pageUrl)">官方页</UiButton>
          <UiButton
            v-if="row.dep.download !== null"
            :variant="row.downloadVariant"
            :disabled="row.downloading"
            :title="row.downloadTitle"
            @click="deps.downloadComponent(row.dep.id, row.downloadUrl)"
          >
            {{ row.downloadLabel }}
          </UiButton>
        </div>
        <!-- 更新提示：发现新版（琥珀 + 更新按钮）→ 已是最新（绿）→ 版本未知（灰）。 -->
        <template v-if="row.hasUpdate">
          <p class="text-xs text-amber-400">{{ row.updateLineText }}</p>
          <UiButton @click="deps.downloadComponent(row.dep.id, row.downloadUrl)"
            >更新到新版</UiButton
          >
        </template>
        <p v-else-if="row.checked" class="text-xs" :class="row.freshClass">{{ row.freshText }}</p>
        <!-- 阶段状态行：失败（红，附原因）→ 下载进度。 -->
        <p v-if="row.failedText !== null" class="text-xs text-red-400">{{ row.failedText }}</p>
        <div v-if="row.downloading" class="flex items-center gap-2 pl-3">
          <div class="h-1.5 overflow-hidden rounded-full bg-zinc-800">
            <div
              class="h-full rounded-full bg-emerald-500 transition-all"
              :style="{ width: row.percentWidth }"
            />
          </div>
          <p class="w-9 text-[10px] tabular-nums text-zinc-400">{{ row.percent }}%</p>
        </div>
        <template v-if="row.downloadedText !== null">
          <p class="text-xs text-emerald-400">{{ row.downloadedText }}</p>
          <p v-if="row.extractDir !== null" class="text-xs text-zinc-500">
            解压目录：{{ row.extractDir }}
          </p>
        </template>
        <p v-if="row.savedText !== null" class="text-xs text-zinc-500">{{ row.savedText }}</p>
        <p v-if="!row.dep.ready && row.dep.required" class="text-xs text-amber-400 pl-3">
          {{ row.dep.hint }}
        </p>
      </template>
    </div>
  </Card>
</template>
