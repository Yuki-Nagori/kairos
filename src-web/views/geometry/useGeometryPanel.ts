/**
 * 几何面板：STL 导入（文件对话框 / 样例）、网格健康摘要与逐几何的
 * 体积网格生成。每个几何行自带目标尺寸 + 引擎（体素 / Gmsh）表单，
 * 初始尺寸取几何最大边的二十分之一；生成动作按引擎分派到对应服务。
 */
import { computed, reactive, watch } from "vue";
import { useAppStore } from "../../stores/app";
import { useGeometryStore } from "../../stores/geometry";
import type { GeometrySummary, MeshIssues, MeshingReport } from "../../types";

export function useGeometryPanel() {
  const app = useAppStore();
  const geometry = useGeometryStore();

  const working = computed(() => app.busy !== null);

  // 每几何行的表单状态（目标尺寸 + 引擎），几何首次出现时以建议尺寸初始化，
  // 之后用户编辑独立于渲染保留（列表重建不回填默认值）。
  interface MeshFormState {
    size: string;
    engine: string;
  }
  const meshForms = reactive<Record<string, MeshFormState>>({});

  watch(
    () => geometry.geometries,
    (geometries) => {
      for (const geometry of geometries) {
        if (meshForms[geometry.geometryId] === undefined) {
          meshForms[geometry.geometryId] = {
            size: (Math.max(...geometry.size) / 20).toPrecision(3),
            engine: "voxel",
          };
        }
      }
    },
    { immediate: true },
  );

  function meshForm(geometry: GeometrySummary): MeshFormState {
    const existing = meshForms[geometry.geometryId];
    if (existing) {
      return existing;
    }
    const created: MeshFormState = {
      size: (Math.max(...geometry.size) / 20).toPrecision(3),
      engine: "voxel",
    };
    meshForms[geometry.geometryId] = created;
    return created;
  }

  const rows = computed(() =>
    geometry.geometries.map((geometry) => ({ geometry, form: meshForm(geometry) })),
  );

  function issueText(issues: MeshIssues): string {
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

  function isClean(geometry: GeometrySummary): boolean {
    const issues = geometry.issues;
    return (
      issues.degenerate === 0 &&
      issues.openEdges === 0 &&
      issues.nonManifoldEdges === 0 &&
      issues.normalInconsistentEdges === 0
    );
  }

  function statsText(geometry: GeometrySummary): string {
    return `${geometry.triangleCount} 三角形 · ${geometry.size
      .map((value) => value.toFixed(2))
      .join(" × ")} ${geometry.suggestedUnit}`;
  }

  function generate(item: GeometrySummary): void {
    const form = meshForm(item);
    const size = Number(form.size);
    void (form.engine === "gmsh"
      ? geometry.generateGmshMesh(item.geometryId, size)
      : geometry.generateMesh(item.geometryId, size));
  }

  function reportText(report: MeshingReport | undefined): string {
    if (!report) {
      return "划分体积网格供求解使用。";
    }
    return `节点 ${report.nodeCount} · 四面体 ${report.elementCount} · 表面 ${report.surfaceFaceCount} · 体积 ${report.totalVolume.toFixed(3)} · 质量比 min ${report.quality.minEdgeRatio.toFixed(2)} / avg ${report.quality.avgEdgeRatio.toFixed(2)} / max ${report.quality.maxEdgeRatio.toFixed(2)}`;
  }

  function onImport(): void {
    void geometry.importGeometry();
  }
  function onSample(): void {
    void geometry.importSampleGeometry(10);
  }

  /** 修复几何：焊接 / 去退化 / 填孔 / 一致化（不健康几何才可用）。 */
  function repair(item: GeometrySummary): void {
    void geometry.repairGeometryById(item.geometryId);
  }
  function repairDisabled(item: GeometrySummary): boolean {
    return working.value || isClean(item);
  }

  return {
    geometry,
    working,
    rows,
    issueText,
    isClean,
    statsText,
    generate,
    reportText,
    onImport,
    onSample,
    repair,
    repairDisabled,
  };
}
