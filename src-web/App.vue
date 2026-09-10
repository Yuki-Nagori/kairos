<script setup lang="ts">
/**
 * 应用根组件：CAE 三列工作台布局在此组合。
 * 各列面板按分析阶段显隐（home 显示全部）；用 v-show 保持挂载，
 * 视口渲染器、图表画布等有状态组件不因切换阶段而重建。
 */
import { useAppStore } from "./stores/app";
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
import PipelinePanel from "./views/pipeline/PipelinePanel.vue";
import MaterialsPanel from "./views/materials/MaterialsPanel.vue";
import GeometryPanel from "./views/geometry/GeometryPanel.vue";
import MoldPanel from "./views/mold/MoldPanel.vue";
import ProcessPanel from "./views/process/ProcessPanel.vue";
import DependenciesPanel from "./views/dependencies/DependenciesPanel.vue";
import ReportPanel from "./views/report/ReportPanel.vue";
import JobsPanel from "./views/jobs/JobsPanel.vue";

const app = useAppStore();

const LEFT_PANELS = [
  { component: ProjectTree, stages: "home,geometry,mesh,process,solve,results,report" },
  { component: PipelinePanel, stages: "home" },
  { component: MaterialsPanel, stages: "home,process" },
  { component: GeometryPanel, stages: "home,geometry,mesh" },
];

const RIGHT_PANELS = [
  { component: MoldPanel, stages: "home,process" },
  { component: ProcessPanel, stages: "home,process" },
  { component: DependenciesPanel, stages: "home,solve" },
  { component: ReportPanel, stages: "home,results,report" },
  { component: JobsPanel, stages: "home,solve,results" },
];

function stageVisible(stages: string): boolean {
  return stages.split(",").includes(app.stage);
}
</script>

<template>
  <!-- 整页锁定不滚动，只有左右列与视口内部各自伸缩 -->
  <div class="flex h-screen flex-col overflow-hidden bg-zinc-950 text-zinc-100">
    <MenuBar />
    <!-- 分析阶段选项卡独立成行：按工作流排序，切换各列面板显隐 -->
    <nav class="flex h-9 shrink-0 items-stretch border-b border-zinc-800 bg-zinc-900 px-2">
      <StageTabs />
    </nav>
    <StageRibbon />
    <main
      class="relative grid min-h-0 flex-1 grid-cols-[280px_minmax(0,1fr)_320px] gap-3 overflow-hidden px-3 py-2"
    >
      <div class="flex min-h-0 flex-col gap-3 overflow-y-auto py-1 pr-1">
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
        <XyChartPanel />
        <LogTabs />
      </div>

      <div class="flex min-h-0 flex-col gap-3 overflow-y-auto py-1 pl-1">
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

    <section class="shrink-0 border-t border-zinc-800 bg-zinc-900 px-4 py-2.5">
      <ResultsPanel />
    </section>
    <StatusBar />
    <CommandPalette />
  </div>
</template>
