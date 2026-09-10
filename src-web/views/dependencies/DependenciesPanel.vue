<script setup lang="ts">
/**
 * 运行时依赖面板：许可分级、就绪状态、应用内下载与官方页引导。
 * 徽标文案与配色正交：许可定颜色（合规口径），安装策略定文案（Gmsh = GPL + 官方直链）。
 * 正文控件顺序沿用原版 DOM append 的最终排布；依赖列表整体由响应式状态一次推导
 * （模板分支在这里算清，等价原生版每次 render 的 replaceChildren 重建）。
 */
import { computed, ref } from "vue";
import { useAppStore } from "../../stores/app";
import { useVmStore } from "../../stores/vm";
import { useDependenciesStore } from "../../stores/dependencies";
import type { DependencyStatus } from "../../types";
import { openDownloadsDir } from "../../api/downloads";
import Card from "../../components/ui/UiCard.vue";
import ShellIcon from "../../components/ui/ShellIcon.vue";
import UiButton from "../../components/ui/UiButton.vue";

/** 徽标：许可定颜色（合规口径），安装策略定文案——两者正交（Gmsh = GPL + 官方直链）。 */
const LICENSE_CLASS: Record<string, string> = {
  mit: "border-emerald-500/60 bg-emerald-500/10 text-emerald-300",
  gpl: "border-amber-500/60 bg-amber-500/10 text-amber-300",
};

const STRATEGY_LABEL: Record<string, string> = {
  direct_download: "官方直链 · 可直接下载",
  guided_install: "引导安装",
};

/** 单行依赖的展示模型：模板分支与文案集中此处算清，避免模板里重复取值。 */
interface DependencyRow {
  dep: DependencyStatus;
  already: boolean;
  downloading: boolean;
  percent: number;
  percentWidth: string;
  downloadUrl: string;
  downloadLabel: string;
  downloadVariant: "primary" | "ghost";
  downloadTitle: string;
  badgeText: string;
  badgeClass: string;
  readyText: string;
  readyClass: string;
  hasUpdate: boolean;
  checked: boolean;
  updateLineText: string;
  freshText: string;
  freshClass: string;
  failedText: string | null;
  downloadedText: string | null;
  extractDir: string | null;
  savedText: string | null;
}

const app = useAppStore();
const vm = useVmStore();
const deps = useDependenciesStore();

/** 取组件在当前平台的下载地址（OS 未就绪时回落 linux 源）。 */
function updateDownloadUrl(dep: DependencyStatus): string {
  const os = app.info?.os ?? "linux";
  return dep.download === null
    ? ""
    : os === "macos"
      ? dep.download.macos
      : os === "windows"
        ? dep.download.windows
        : dep.download.linux;
}

const rows = computed<DependencyRow[]>(() =>
  deps.dependencies.map((dep) => {
    const stage = deps.componentStages[dep.id];
    const downloading = stage?.stage === "downloading";
    const percent = stage?.stage === "downloading" ? stage.percent : 0;
    const already = dep.id in deps.savedDownloads || dep.id in deps.downloadedFiles;
    const check = deps.updateChecks[dep.id];
    const downloaded = deps.downloadedFiles[dep.id];
    const saved = deps.savedDownloads[dep.id];
    const usable = dep.ready || dep.managedReady;
    const sizeMb = downloaded !== undefined ? (downloaded.sizeBytes / 1024 / 1024).toFixed(1) : "";
    return {
      dep,
      already,
      downloading,
      percent,
      percentWidth: `${Math.min(Math.max(percent, 0), 100)}%`,
      downloadUrl: updateDownloadUrl(dep),
      downloadLabel: already ? "重新下载" : "下载",
      downloadVariant: already ? "ghost" : "primary",
      downloadTitle: `下载（${dep.license}）`,
      badgeText: STRATEGY_LABEL[dep.strategy] ?? dep.license,
      badgeClass: LICENSE_CLASS[dep.licenseKind] ?? "border-zinc-700 text-zinc-400",
      readyText: usable ? (dep.managedReady ? "就绪（应用内副本）" : "就绪") : "未就绪",
      readyClass: usable ? "text-emerald-400" : "text-red-400",
      hasUpdate: check?.updateAvailable === true,
      checked: check !== undefined,
      updateLineText:
        check?.updateAvailable === true
          ? `发现新版本 ${check.latestTag}（当前 ${check.installedTag ?? "未知"}）`
          : "",
      freshText:
        check !== undefined && check.updateAvailable !== true
          ? check.installedTag === null
            ? "已安装（版本标识未知，重新下载可获得版本标记）。"
            : `已是最新版本（${check.installedTag}）。`
          : "",
      freshClass:
        check !== undefined && check.updateAvailable !== true
          ? `text-xs ${check.installedTag === null ? "text-zinc-400" : "text-emerald-400"}`
          : "",
      failedText: stage?.stage === "failed" ? stage.error : null,
      downloadedText:
        downloaded !== undefined
          ? `已下载 ${downloaded.fileName}（${sizeMb} MB${downloaded.extractDir !== null ? "，已解压" : ""}）`
          : null,
      extractDir: downloaded?.extractDir ?? null,
      savedText:
        saved !== undefined
          ? saved.extractDir !== null
            ? `已解压：${saved.extractDir}`
            : `已保存：${saved.path}`
          : null,
    };
  }),
);

// 下载目录路径异步取回；动态 import 沿用原版（探测前展示占位文案）。
const downloadsDir = ref("探测中…");
void import("../../api/downloads").then(async (m) => {
  downloadsDir.value = await m.getDownloadsDir();
});

void deps.refreshDependencies();
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
          <!-- 有应用内下载地址的组件：用户点击触发（官方源 + 许可展示）；下载进行中禁用。 -->
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
