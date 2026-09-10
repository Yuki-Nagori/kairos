import "./app.css";
import { createAppHeader, createStatusBar } from "./components/app-header";
import { createStageTabs } from "./components/stage-tabs";
import { setupMenuActions } from "./menu-actions";
import { createDependenciesPanel } from "./components/panels/dependencies-panel";
import { createVmDock } from "./components/vm-dock";
import { createGeometryPanel } from "./components/panels/geometry-panel";
import { createJobsPanel } from "./components/panels/jobs-panel";
import { createMaterialsPanel } from "./components/panels/materials-panel";
import { createMoldPanel } from "./components/panels/mold-panel";
import { createPipelinePanel } from "./components/panels/pipeline-panel";
import { createProcessPanel } from "./components/panels/process-panel";
import { createProjectTree } from "./components/panels/project-tree";
import { createReportPanel } from "./components/panels/report-panel";
import { createResultsPanel } from "./components/panels/results-panel";
import { createViewportPanel } from "./components/panels/viewport-panel";
import { createXyChartPanel } from "./components/panels/xy-chart-panel";
import { setupGlobalShortcuts } from "./shortcuts";
import { initTheme } from "./theme";
import { appStore, bootstrap } from "./state";

const root = document.querySelector<HTMLDivElement>("#app");

if (!root) {
  throw new Error("Root element #app not found");
}

// CAE 三列工作台（对标 ui.html）：整页锁定不滚动，只有左右列与视口内部各自伸缩。
root.className = "flex h-screen flex-col overflow-hidden bg-zinc-950 text-zinc-100";

// 先恢复主题再创建组件，标题栏的初始主题图标才能与持久化偏好一致。
initTheme();

const header = createAppHeader();

const leftColumn = document.createElement("div");
leftColumn.className = "flex min-h-0 flex-col gap-3 overflow-y-auto py-1 pr-1";
// 流程引导属工作流辅助，放左列（可折叠）；中列留给视口与图表。
// 滚动列里的卡片必须 shrink-0：宁可列滚动，也不让卡片内容被压缩裁切。
const stagePanel = (el: HTMLElement, stages: string): HTMLElement => {
  el.dataset.stages = stages;
  el.classList.add("stage-panel", "shrink-0");
  return el;
};
for (const panel of [
  stagePanel(createProjectTree(), "home,geometry,mesh,process,solve,results,report"),
  stagePanel(createPipelinePanel(), "home"),
  stagePanel(createMaterialsPanel(), "home,process"),
  stagePanel(createGeometryPanel(), "home,geometry,mesh"),
]) {
  leftColumn.append(panel);
}

const centerColumn = document.createElement("div");
centerColumn.className = "flex min-h-0 min-w-0 flex-col gap-3 overflow-hidden";
centerColumn.append(createViewportPanel(), createXyChartPanel());

const rightColumn = document.createElement("div");
rightColumn.className = "flex min-h-0 flex-col gap-3 overflow-y-auto py-1 pl-1";
for (const panel of [
  stagePanel(createMoldPanel(), "home,process"),
  stagePanel(createProcessPanel(), "home,process"),
  stagePanel(createDependenciesPanel(), "home,solve"),
  stagePanel(createReportPanel(), "home,results,report"),
  stagePanel(createJobsPanel(), "home,solve,results"),
]) {
  rightColumn.append(panel);
}

const workspace = document.createElement("main");
workspace.className =
  "relative grid min-h-0 flex-1 grid-cols-[280px_minmax(0,1fr)_320px] gap-3 overflow-hidden px-3 py-2";
workspace.append(leftColumn, centerColumn, rightColumn);

// 虚拟机 / Shell 面板：浮于工作区右下角（终端抽屉式，不占布局列）。
workspace.append(createVmDock());

const resultsSection = document.createElement("section");
resultsSection.className = "shrink-0 border-t border-zinc-800 bg-zinc-900 px-4 py-2.5";
resultsSection.append(createResultsPanel());

const statusBar = createStatusBar();

root.append(header, createStageTabs(), workspace, resultsSection, statusBar);

// 分析阶段切换：按 data-stages 显隐面板（home 显示全部）
const syncStages = (): void => {
  const stage = appStore.get().stage;
  document.querySelectorAll<HTMLElement>(".stage-panel").forEach((el) => {
    const stages = (el.dataset.stages ?? "").split(",");
    el.classList.toggle("hidden", !stages.includes(stage));
  });
};
syncStages();
appStore.subscribe(syncStages);

setupGlobalShortcuts();
setupMenuActions();

// bootstrap 内部已自行处理失败（setError），无需 await。
void bootstrap();
