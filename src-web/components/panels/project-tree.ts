import { appStore } from "../../state";

/** 项目树面板：层级展示工程 / 研究 / 几何 / 网格 / 材料 / 工艺 / 结果。 */
export function createProjectTree(): HTMLElement {
  const panel = document.createElement("aside");
  panel.className = "project-tree";

  const header = document.createElement("div");
  header.className = "tree-header";
  header.textContent = "工程浏览器";

  const tree = document.createElement("div");
  tree.className = "tree-body";

  function render(): void {
    const s = appStore.get();
    tree.replaceChildren();

    tree.append(group("项目"));
    if (s.project) {
      tree.append(leaf(s.project.name));
    } else {
      tree.append(leaf("未打开项目"));
    }

    if (s.geometries.length > 0) {
      tree.append(group("几何"));
      for (const geo of s.geometries) {
        tree.append(leaf(`${geo.fileName} (${geo.triangleCount} 面)`));
      }
    }

    if (s.jobs.length > 0) {
      tree.append(group("求解作业"));
      for (const job of s.jobs) {
        tree.append(leaf(`${job.id}: ${job.status}`));
      }
    }

    if (s.dependencies.length > 0) {
      tree.append(group("运行时依赖"));
      for (const dep of s.dependencies) {
        tree.append(leaf(`${dep.name}: ${dep.ready ? "就绪" : "未就绪"}`));
      }
    }
  }

  function group(title: string): HTMLElement {
    const el = document.createElement("div");
    el.className = "tree-group";
    el.textContent = title;
    return el;
  }

  function leaf(label: string): HTMLElement {
    const el = document.createElement("div");
    el.className = "tree-leaf";
    el.textContent = label;
    return el;
  }

  panel.append(header, tree);
  render();
  appStore.subscribe(render);
  return panel;
}
