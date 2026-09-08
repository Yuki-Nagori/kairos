import {
  appStore,
  generateMesh,
  importGeometry,
  importSampleGeometry,
  removeGeometryById,
} from "../../state";
import type { MeshingReport } from "../../types";
import { button, card, hint, numberInput } from "../ui";

/** 几何面板：STL 导入、健康检查、体积网格生成与已导入几何列表。 */
export function createGeometryPanel(): HTMLElement {
  const { root, body } = card("几何");

  const actions = document.createElement("div");
  actions.className = "flex flex-wrap items-center gap-2";
  const importButton = button("导入 STL", "primary");
  const sampleButton = button("导入样例");
  actions.append(importButton, sampleButton);

  const listBox = document.createElement("div");
  listBox.className = "space-y-2";

  importButton.addEventListener("click", () => void importGeometry());
  sampleButton.addEventListener("click", () => void importSampleGeometry(10));

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
    const { geometries, busy, meshReports } = appStore.get();
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
      issues.textContent = clean ? "网格健康" : issueText(geometry.issues);

      const remove = button("移除", "danger");
      remove.disabled = working;
      remove.addEventListener("click", () => {
        void removeGeometryById(geometry.geometryId);
      });

      row.append(name, stats, issues, remove);

      // 网格生成区：目标尺寸输入 + 生成按钮 + 报告
      const meshRow = document.createElement("div");
      meshRow.className = "flex w-full flex-wrap items-center gap-2 border-t border-zinc-800 pt-2";

      const sizeInput = numberInput("目标尺寸", "w-28");
      sizeInput.step = "any";
      sizeInput.min = "0";
      sizeInput.value = (Math.max(...geometry.size) / 20).toPrecision(3);
      const generateButton = button("生成体积网格");
      generateButton.disabled = working;
      const reportLine = hint(reportText(meshReports[geometry.geometryId]));

      generateButton.addEventListener("click", () => {
        void generateMesh(geometry.geometryId, Number(sizeInput.value));
      });

      meshRow.append(sizeInput, generateButton, reportLine);
      row.append(meshRow);
      listBox.append(row);
    }
  }

  body.append(actions, listBox);
  render();
  appStore.subscribe(render);
  return root;
}

function reportText(report: MeshingReport | undefined): string {
  if (!report) {
    return "划分体积网格供求解使用。";
  }
  return `节点 ${report.nodeCount} · 四面体 ${report.elementCount} · 表面 ${report.surfaceFaceCount} · 体积 ${report.totalVolume.toFixed(3)} · 质量比 min ${report.quality.minEdgeRatio.toFixed(2)} / avg ${report.quality.avgEdgeRatio.toFixed(2)} / max ${report.quality.maxEdgeRatio.toFixed(2)}`;
}
