/**
 * 几何面板：STL 导入（文件对话框 / 样例）、网格健康摘要与逐几何的
 * 体积网格生成。每个几何行自带目标尺寸 + 引擎（体素 / Gmsh）表单，
 * 初始尺寸取几何最大边的二十分之一；生成动作按引擎分派到对应服务。
 */
import { computed, reactive, watch } from "vue";
import { useAppStore } from "../../stores/app";
import { useGeometryStore } from "../../stores/geometry";
import type {
  DualDomainReport,
  GeometrySummary,
  MeshIssues,
  MeshingReport,
  MidplaneReport,
  RepairReport,
} from "../../types";

export function useGeometryPanel() {
  const app = useAppStore();
  const geometry = useGeometryStore();

  const working = computed(() => app.busy !== null);

  // 每几何行的表单状态（目标尺寸 + 引擎），几何首次出现时以建议尺寸初始化，
  // 之后用户编辑独立于渲染保留（列表重建不回填默认值）。
  interface MeshFormState {
    size: string;
    engine: string;
    /** 体素引擎可选边界层层数；空串 = 不分级。 */
    boundaryLayers: string;
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
            boundaryLayers: "",
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
      boundaryLayers: "",
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
    if (form.engine === "gmsh") {
      void geometry.generateGmshMesh(item.geometryId, size);
      return;
    }
    const layers = Number(form.boundaryLayers);
    const refinement =
      form.engine === "voxel" && layers >= 1
        ? ({ mode: "boundaryLayers", layers, ratio: 0.5 } as const)
        : undefined;
    void geometry.generateMesh(item.geometryId, size, refinement);
  }

  /** 网格尺寸与最小特征的匹配提示（网格报告里的警告行）。 */
  function thinFeatureHints(report: MeshingReport | undefined): string[] {
    return report?.thinFeatureHints ?? [];
  }

  function reportText(report: MeshingReport | undefined): string {
    if (!report) {
      return "划分体积网格供求解使用。";
    }
    return `节点 ${report.nodeCount} · 四面体 ${report.elementCount} · 表面 ${report.surfaceFaceCount} · 体积 ${report.totalVolume.toFixed(3)} · 质量比 min ${report.quality.minEdgeRatio.toFixed(2)} / avg ${report.quality.avgEdgeRatio.toFixed(2)} / max ${report.quality.maxEdgeRatio.toFixed(2)}`;
  }

  function dualReportText(report: DualDomainReport | undefined): string {
    if (!report) {
      return "表面厚度配对 + 杆系梁耦合（2.5D 快速分析路线）。";
    }
    const thickness = report.thicknessMin.toFixed(2);
    return `三角形 ${report.triangleCount} · 厚度 ${thickness} ~ ${report.thicknessMax.toFixed(2)}（avg ${report.thicknessAvg.toFixed(2)}）· 未配对 ${report.unpairedTriangles} · 梁 ${report.beamCount}（耦合 ${report.couplingCount} / 自由 ${report.uncoupledEndpoints}）`;
  }

  function midplaneReportText(report: MidplaneReport | undefined): string {
    if (!report) {
      return "顶点配对中面抽取（1D/2.5D 快速分析路线）。";
    }
    return `单元 ${report.elementCount} · 节点 ${report.nodeCount} · 厚度 ${report.thicknessMin.toFixed(2)} ~ ${report.thicknessMax.toFixed(2)}（avg ${report.thicknessAvg.toFixed(2)}）· 丢弃 ${report.droppedElements} · 梁 ${report.beamCount}（耦合 ${report.couplingCount}）`;
  }

  /** 修复报告：只列非零修复项；自交为纯检测项，始终展示计数。 */
  function repairReportText(report: RepairReport | undefined): string {
    if (!report) {
      return "尚未修复。";
    }
    const parts: string[] = [];
    if (report.mergedVertices > 0) {
      parts.push(`焊接顶点 ${report.mergedVertices}`);
    }
    if (report.removedDegenerate > 0) {
      parts.push(`去退化 ${report.removedDegenerate}`);
    }
    if (report.filledHoles > 0) {
      parts.push(`填孔 ${report.filledHoles}（+${report.filledTriangles} 面）`);
    }
    if (report.flippedFaces > 0) {
      parts.push(`翻转法向 ${report.flippedFaces}`);
    }
    parts.push(`自交 ${report.selfIntersections}`);
    return `修复：${parts.join(" · ")}`;
  }

  /** 中面网格：按当前方案流道/浇口做杆系耦合。 */
  function generateMid(item: GeometrySummary): void {
    void geometry.generateMidplane(item.geometryId);
  }

  function onImport(): void {
    void geometry.importGeometry();
  }
  function onSample(): void {
    void geometry.importSampleGeometry(10);
  }

  /** 双域网格：按当前方案流道/浇口做杆系耦合。 */
  function generateDual(item: GeometrySummary): void {
    void geometry.generateDualDomain(item.geometryId);
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
    thinFeatureHints,
    dualReportText,
    repairReportText,
    generateDual,
    midplaneReportText,
    generateMid,
    onImport,
    onSample,
    repair,
    repairDisabled,
  };
}
