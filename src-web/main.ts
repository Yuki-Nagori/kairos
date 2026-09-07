import "./app.css";
import { createAppHeader } from "./components/app-header";
import { createDependenciesPanel } from "./components/dependencies-panel";
import { createGeometryPanel } from "./components/geometry-panel";
import { createJobsPanel } from "./components/jobs-panel";
import { createMaterialsPanel } from "./components/materials-panel";
import { createMoldPanel } from "./components/mold-panel";
import { createPipelinePanel } from "./components/pipeline-panel";
import { createProcessPanel } from "./components/process-panel";
import { createProjectTree } from "./components/project-tree";
import { createReportPanel } from "./components/report-panel";
import { createResultsPanel } from "./components/results-panel";
import { createViewportPanel } from "./components/viewport-panel";
import { createXyChartPanel } from "./components/xy-chart-panel";
import { initTheme } from "./theme";
import { bootstrap, newProject, openProject, saveProject } from "./state";

const root = document.querySelector<HTMLDivElement>("#app");

if (!root) {
  throw new Error("Root element #app not found");
}

// CAE 三列工作台：左（工程与研究）/ 中（流程与视口）/ 右（模具与工艺作业），底部结果。
root.className = "flex h-screen flex-col overflow-hidden bg-zinc-950 text-zinc-100";

// 先恢复主题再创建组件，标题栏的初始主题图标才能与持久化偏好一致。
initTheme();

const header = createAppHeader();

const leftColumn = document.createElement("div");
leftColumn.className = "flex w-[320px] shrink-0 flex-col gap-3 overflow-y-auto p-3";
leftColumn.append(createProjectTree(), createMaterialsPanel(), createGeometryPanel());

const centerColumn = document.createElement("div");
centerColumn.className = "flex min-w-0 flex-1 flex-col gap-3 overflow-y-auto p-3";
centerColumn.append(createPipelinePanel(), createViewportPanel(), createXyChartPanel());

const rightColumn = document.createElement("div");
rightColumn.className = "flex w-[340px] shrink-0 flex-col gap-3 overflow-y-auto p-3";
rightColumn.append(
  createMoldPanel(),
  createProcessPanel(),
  createDependenciesPanel(),
  createReportPanel(),
  createJobsPanel(),
);

const workspace = document.createElement("main");
workspace.className =
  "grid flex-1 grid-cols-[320px_minmax(0,1fr)_340px] gap-3 overflow-hidden px-3";
workspace.append(leftColumn, centerColumn, rightColumn);

const resultsSection = document.createElement("section");
resultsSection.className = "border-t border-zinc-800 bg-zinc-900 px-3 py-3";
resultsSection.append(createResultsPanel());

const bottomBar = document.createElement("footer");
bottomBar.className =
  "flex items-center justify-between border-t border-zinc-800 px-4 py-1.5 text-[11px] text-zinc-500";

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

root.append(header, workspace, resultsSection, bottomBar);

// bootstrap 内部已自行处理失败（setError），无需 await。
void bootstrap();
