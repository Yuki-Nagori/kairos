import "./app.css";
import { createAppHeader } from "./components/app-header";
import { createGeometryPanel } from "./components/geometry-panel";
import { createMoldPanel } from "./components/mold-panel";
import { createMaterialsPanel } from "./components/materials-panel";
import { createProcessPanel } from "./components/process-panel";
import { createProjectBar } from "./components/project-bar";
import { createWorkspace } from "./components/workspace";
import { bootstrap } from "./state";

const root = document.querySelector<HTMLDivElement>("#app");

if (!root) {
  throw new Error("Root element #app not found");
}

root.className = "mx-auto flex min-h-screen max-w-6xl flex-col gap-6 px-6 py-8";

root.append(
  createAppHeader(),
  createProjectBar(),
  createGeometryPanel(),
  createMaterialsPanel(),
  createMoldPanel(),
  createProcessPanel(),
  createWorkspace(),
);

// bootstrap 内部已自行处理失败（setError），无需 await。
void bootstrap();
