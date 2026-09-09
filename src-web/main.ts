import "./app.css";
import { createAppHeader, createStatusBar } from "./components/app-header";
import { setupMenuActions } from "./menu-actions";
import { createDependenciesPanel } from "./components/panels/dependencies-panel";
import { createVmPanel } from "./components/panels/vm-panel";
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
import { initTheme } from "./theme";
import { appStore, bootstrap, newProject, openProject, saveProject } from "./state";

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
for (const panel of [
  createProjectTree(),
  createPipelinePanel(),
  createMaterialsPanel(),
  createGeometryPanel(),
]) {
  panel.classList.add("shrink-0");
  leftColumn.append(panel);
}

const centerColumn = document.createElement("div");
centerColumn.className = "flex min-h-0 min-w-0 flex-col gap-3 overflow-hidden";
centerColumn.append(createViewportPanel(), createXyChartPanel());

const rightColumn = document.createElement("div");
rightColumn.className = "flex min-h-0 flex-col gap-3 overflow-y-auto py-1 pl-1";
for (const panel of [
  createMoldPanel(),
  createProcessPanel(),
  createDependenciesPanel(),
  createReportPanel(),
  createJobsPanel(),
]) {
  panel.classList.add("shrink-0");
  rightColumn.append(panel);
}

const workspace = document.createElement("main");
workspace.className =
  "relative grid min-h-0 flex-1 grid-cols-[280px_minmax(0,1fr)_320px] gap-3 overflow-hidden px-3 py-2";
workspace.append(leftColumn, centerColumn, rightColumn);

// 虚拟机 / Shell 面板：浮于工作区右下角（终端抽屉式，不占布局列）。
const vmDock = document.createElement("div");
vmDock.className = "absolute bottom-2 right-2 z-40 w-[26rem] shadow-2xl shadow-black/50";
vmDock.append(createVmPanel());
workspace.append(vmDock);
// 显隐由状态栏右侧 Shell 按钮控制（默认隐藏，点击出现）。
const syncVmDock = (): void => {
  vmDock.classList.toggle("hidden", !appStore.get().vmPanelVisible);
};
syncVmDock();
appStore.subscribe(syncVmDock);

const resultsSection = document.createElement("section");
resultsSection.className = "shrink-0 border-t border-zinc-800 bg-zinc-900 px-4 py-2.5";
resultsSection.append(createResultsPanel());

const statusBar = createStatusBar();

// 全局快捷键（CAD 习惯）：Ctrl/Cmd+S 保存、Ctrl/Cmd+O 打开、Ctrl/Cmd+N 新建
window.addEventListener("keydown", (event) => {
  if (!(event.ctrlKey || event.metaKey)) {
    return;
  }
  const key = event.key.toLowerCase();
  if (key === "s") {
    event.preventDefault();
    void saveProject();
  } else if (key === "o") {
    event.preventDefault();
    void openProject();
  } else if (key === "n") {
    event.preventDefault();
    void newProject("未命名项目");
  }
});

root.append(header, workspace, resultsSection, statusBar);

setupMenuActions();

// bootstrap 内部已自行处理失败（setError），无需 await。
void bootstrap();
