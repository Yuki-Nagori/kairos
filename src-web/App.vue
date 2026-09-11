<script setup lang="ts">
/**
 * 应用根组件：CAE 三列工作台布局在此组合。
 * 各列面板按分析阶段显隐（home 显示全部）；用 v-show 保持挂载，
 * 视口渲染器、图表画布等有状态组件不因切换阶段而重建。
 * 左右列可折叠（useLayout），中列视口恒驻。
 */
import { computed } from "vue";
import type { Component } from "vue";
import { useAppStore } from "./stores/app";
import { useLayout } from "./composables/useLayout";
import { STAGES } from "./components/stage-tabs/useStageTabs";
import MenuBar from "./components/menu-bar/MenuBar.vue";
import StageTabs from "./components/stage-tabs/StageTabs.vue";
import StageRibbon from "./components/stage-ribbon/StageRibbon.vue";
import CommandPalette from "./components/command-palette/CommandPalette.vue";
import StatusBar from "./components/status-bar/StatusBar.vue";
import LogTabs from "./components/log-tabs/LogTabs.vue";
import VmDock from "./components/vm-dock/VmDock.vue";
import VmPanel from "./views/vm/VmPanel.vue";
import ViewportPanel from "./views/viewport/ViewportPanel.vue";
import XyChartPanel from "./views/xy-chart/XyChartPanel.vue";
import ResultsPanel from "./views/results/ResultsPanel.vue";
import ProjectTree from "./views/project-tree/ProjectTree.vue";
import LayersPanel from "./views/layers/LayersPanel.vue";
import PipelinePanel from "./views/pipeline/PipelinePanel.vue";
import MaterialsPanel from "./views/materials/MaterialsPanel.vue";
import GeometryPanel from "./views/geometry/GeometryPanel.vue";
import MoldPanel from "./views/mold/MoldPanel.vue";
import ProcessPanel from "./views/process/ProcessPanel.vue";
import DependenciesPanel from "./views/dependencies/DependenciesPanel.vue";
import ReportPanel from "./views/report/ReportPanel.vue";
import JobsPanel from "./views/jobs/JobsPanel.vue";
import type { Stage } from "./types";

/** 列内面板配置：component 恒挂载，stages 决定可见的分析阶段。 */
interface PanelConfig {
  component: Component;
  stages: Stage[];
}

const app = useAppStore();
const { leftCollapsed, rightCollapsed, toggleLeft, toggleRight } = useLayout();

const ALL_STAGES: Stage[] = STAGES.map(([stage]) => stage);

const LEFT_PANELS: PanelConfig[] = [
  { component: ProjectTree, stages: ALL_STAGES },
  { component: LayersPanel, stages: ALL_STAGES },
  { component: PipelinePanel, stages: ["home"] },
  { component: MaterialsPanel, stages: ["home", "process"] },
  { component: GeometryPanel, stages: ["home", "geometry", "mesh"] },
];

const RIGHT_PANELS: PanelConfig[] = [
  { component: MoldPanel, stages: ["home", "process"] },
  { component: ProcessPanel, stages: ["home", "process"] },
  { component: DependenciesPanel, stages: ["home", "solve"] },
  { component: ResultsPanel, stages: ["results", "report"] },
  { component: ReportPanel, stages: ["home", "results", "report"] },
  { component: JobsPanel, stages: ["home", "solve", "results"] },
];

function stageVisible(stages: Stage[]): boolean {
  return stages.includes(app.stage);
}

/** 三列网格列宽随折叠状态切换（Tailwind 任意值类需整串出现在源码中）。 */
const gridClass = computed(() => {
  if (leftCollapsed.value && rightCollapsed.value) {
    return "grid-cols-[0px_minmax(0,1fr)_0px]";
  }
  if (leftCollapsed.value) {
    return "grid-cols-[0px_minmax(0,1fr)_320px]";
  }
  if (rightCollapsed.value) {
    return "grid-cols-[280px_minmax(0,1fr)_0px]";
  }
  return "grid-cols-[280px_minmax(0,1fr)_320px]";
});
</script>
<template>
  <!-- 整页锁定不滚动，只有左右列与视口内部各自伸缩 -->
  <div class="flex h-screen flex-col overflow-hidden bg-zinc-950 text-zinc-100">
    <MenuBar />
    <!-- 分析阶段选项卡独立成行：按工作流排序，行尾为左右列折叠开关 -->
    <nav class="flex h-9 shrink-0 items-stretch border-b border-zinc-800 bg-zinc-900 px-2">
      <StageTabs />
      <span class="flex-1" />
      <button
        type="button"
        class="px-2 text-[11px] text-zinc-500 transition-colors hover:text-zinc-200"
        :title="leftCollapsed ? '展开左列' : '收起左列'"
        @click="toggleLeft()"
      >
        {{ leftCollapsed ? "⏵" : "⏴" }}
      </button>
      <button
        type="button"
        class="px-2 text-[11px] text-zinc-500 transition-colors hover:text-zinc-200"
        :title="rightCollapsed ? '展开右列' : '收起右列'"
        @click="toggleRight()"
      >
        {{ rightCollapsed ? "⏴" : "⏵" }}
      </button>
    </nav>
    <StageRibbon />
    <main
      class="relative grid min-h-0 flex-1 gap-3 overflow-hidden px-3 py-2 transition-[grid-template-columns]"
      :class="gridClass"
    >
      <div v-show="!leftCollapsed" class="flex min-h-0 flex-col gap-3 overflow-y-auto py-1 pr-1">
        <!-- 滚动列里的卡片必须 shrink-0：宁可列滚动，也不让卡片内容被压缩裁切 -->
        <component
          :is="panel.component"
          v-for="(panel, index) in LEFT_PANELS"
          :key="index"
          v-show="stageVisible(panel.stages)"
          class="shrink-0"
        />
      </div>

      <div class="flex min-h-0 min-w-0 flex-col gap-3 overflow-hidden">
        <ViewportPanel />
        <!-- XY 曲线是结果阶段工具：结果 / 主页可见，其他阶段让位给视口与日志 -->
        <XyChartPanel v-show="stageVisible(['home', 'results'])" />
        <LogTabs />
      </div>

      <div v-show="!rightCollapsed" class="flex min-h-0 flex-col gap-3 overflow-y-auto py-1 pl-1">
        <component
          :is="panel.component"
          v-for="(panel, index) in RIGHT_PANELS"
          :key="index"
          v-show="stageVisible(panel.stages)"
          class="shrink-0"
        />
      </div>

      <!-- 虚拟机 / Shell 面板：浮于工作区右下角（终端抽屉式，不占布局列） -->
      <VmDock>
        <VmPanel />
      </VmDock>
    </main>
    <StatusBar />
    <CommandPalette />
  </div>
</template>
