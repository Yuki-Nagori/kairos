import { appStore, importGeometry, removeGeometryById } from "../state";
import { button, card, hint } from "./ui";

/** 几何面板：STL 导入、健康检查结果与已导入几何列表。 */
export function createGeometryPanel(): HTMLElement {
  const { root, body } = card("几何");

  const actions = document.createElement("div");
  actions.className = "flex flex-wrap items-center gap-2";
  const importButton = button("导入 STL", "primary");
  actions.append(importButton);

  const listBox = document.createElement("div");
  listBox.className = "space-y-2";

  importButton.addEventListener("click", () => void importGeometry());

  function issueText(issues: {
    degenerate: number;
    openEdges: number;
    nonManifoldEdges: number;
    normalInconsistentEdges: number;
  }): string {
    const parts: string[] = [];
    if (issues.openEdges > 0) {
      parts.push(`开放边 ${issues.openEdges}`);
    }
    if (issues.degenerate > 0) {
      parts.push(`退化三角形 ${issues.degenerate}`);
    }
    if (issues.nonManifoldEdges > 0) {
      parts.push(`非流形边 ${issues.nonManifoldEdges}`);
    }
    if (issues.normalInconsistentEdges > 0) {
      parts.push(`法向不一致 ${issues.normalInconsistentEdges}`);
    }
    return parts.length > 0 ? parts.join("，") : "网格健康";
  }

  function render(): void {
    const { geometries, busy } = appStore.get();
    const working = busy !== null;
    importButton.disabled = working;

    listBox.replaceChildren();
    if (geometries.length === 0) {
      listBox.append(hint("尚未导入几何。支持二进制 / ASCII STL。"));
    }
    for (const geometry of geometries) {
      const row = document.createElement("div");
      row.className =
        "flex flex-wrap items-center gap-x-4 gap-y-1 rounded-lg border border-zinc-800 px-3 py-2 text-xs";

      const name = document.createElement("span");
      name.className = "font-semibold text-zinc-200";
      name.textContent = geometry.fileName;

      const stats = document.createElement("span");
      stats.className = "text-zinc-500";
      stats.textContent = `${geometry.triangleCount} 三角形 · ${geometry.size
        .map((value) => value.toFixed(2))
        .join(" × ")} ${geometry.suggestedUnit}`;

      const clean =
        geometry.issues.degenerate === 0 &&
        geometry.issues.openEdges === 0 &&
        geometry.issues.nonManifoldEdges === 0 &&
        geometry.issues.normalInconsistentEdges === 0;
      const issues = hint(issueText(geometry.issues));
      issues.className = clean ? "text-emerald-400" : "text-amber-400";
      issues.textContent = clean ? "网格健康" : `⚠ ${issueText(geometry.issues)}`;

      const remove = button("移除", "danger");
      remove.disabled = working;
      remove.addEventListener("click", () => {
        void removeGeometryById(geometry.geometryId);
      });

      row.append(name, stats, issues, remove);
      listBox.append(row);
    }
  }

  body.append(actions, listBox);
  render();
  appStore.subscribe(render);
  return root;
}
