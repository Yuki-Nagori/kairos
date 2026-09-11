/** 报告面板：汇总项目/材料/工艺/几何/结果快照/探针与时间序列，生成自包含 HTML 报告。
 * 模板化：自定义标题 / 备注 / 分区开关（分区全开为默认模板）。 */
import { reactive, ref } from "vue";
import { useGeometryStore } from "../../stores/geometry";
import { useMaterialsStore } from "../../stores/materials";
import { useProjectStore } from "../../stores/project";
import { useResultsStore } from "../../stores/results";
import { buildReportHtml, type ReportOptions } from "../../utils/report";
import { minMax } from "../../utils/stats";
import { getSnapshotDataUrl } from "../../render/snapshot";

export function useReportPanel() {
  const project = useProjectStore();
  const materials = useMaterialsStore();
  const results = useResultsStore();
  const geometry = useGeometryStore();

  // 状态行三种结局：初始引导语 → 缺项目/研究 → 生成成功。
  const status = ref("生成自包含 HTML（浏览器打开后 Ctrl+P 打印为 PDF）。");

  // 报告模板：自定义标题（空 = 默认）、备注、分区开关。
  const template = reactive({
    title: "",
    notes: "",
    sections: {
      parameters: true,
      geometry: true,
      fieldStats: true,
      probes: true,
      timeSeries: true,
      snapshots: true,
    },
  });

  function generateReport(): void {
    const currentProject = project.project;
    const materialsLib = materials.materials;
    const loadedField = results.loadedField;
    const study = project.activeStudy;
    if (currentProject === null || study === null) {
      status.value = "请先创建项目与研究。";
      return;
    }
    const material = study.materialId
      ? [...materialsLib.builtin, ...materialsLib.custom].find((m) => m.id === study.materialId)
      : undefined;
    const rows: Array<[string, string]> = [
      ["材料", material ? `${material.manufacturer} · ${material.name}` : "未登记"],
      ["熔体温度", study.process ? `${study.process.meltTempC} °C` : "未设置"],
      ["模具温度", study.process ? `${study.process.moldTempC} °C` : "未设置"],
      ["注射时间", study.process ? `${study.process.injectionTimeS} s` : "未设置"],
      ["保压时间", study.process ? `${study.process.packingTimeS} s` : "未设置"],
      ["冷却时间", study.process ? `${study.process.coolingTimeS} s` : "未设置"],
    ];
    const snapshots: Array<{ title: string; dataUrl: string }> = [];
    const viewport = getSnapshotDataUrl("viewport");
    if (viewport !== null) {
      snapshots.push({ title: "视口", dataUrl: viewport });
    }
    const chart = getSnapshotDataUrl("xy-chart");
    if (chart !== null) {
      snapshots.push({ title: "XY 曲线", dataUrl: chart });
    }
    let fieldStats: string | null = null;
    if (loadedField !== null && loadedField.values.length > 0) {
      const { min, max } = minMax(loadedField.values);
      fieldStats = `${loadedField.field} @ ${loadedField.timeDir}s：${loadedField.values.length} 个值，min ${min.toFixed(3)} / max ${max.toFixed(3)}${loadedField.complete ? "" : "（不完整）"}`;
    }

    // 几何摘要：首个已导入几何的规模与健康度；体积网格报告存在时附质量行。
    const geometryRows: Array<[string, string]> = [];
    const geometrySummary = geometry.geometries[0];
    if (geometrySummary !== undefined) {
      const issues = geometrySummary.issues;
      const healthy =
        issues.degenerate === 0 &&
        issues.openEdges === 0 &&
        issues.nonManifoldEdges === 0 &&
        issues.normalInconsistentEdges === 0;
      geometryRows.push(
        ["三角形数", String(geometrySummary.triangleCount)],
        [
          "尺寸",
          `${geometrySummary.size.map((value) => value.toFixed(2)).join(" × ")} ${geometrySummary.suggestedUnit}`,
        ],
        [
          "网格健康",
          healthy
            ? "健康"
            : `退化 ${issues.degenerate} / 开放边 ${issues.openEdges} / 非流形 ${issues.nonManifoldEdges}`,
        ],
      );
    }
    const meshReport = geometry.meshReports[geometrySummary?.geometryId ?? ""];
    if (meshReport !== undefined) {
      geometryRows.push([
        "体积网格",
        `${meshReport.engine} · 节点 ${meshReport.nodeCount} · 四面体 ${meshReport.elementCount} · 体积 ${meshReport.totalVolume.toFixed(3)}`,
      ]);
    }

    // 探针数值：当前加载场下各探针的取值。
    const probeRows: Array<[string, string]> = loadedField
      ? results.probes.map((probe) => [
          `#${probe.id} · 节点 ${probe.nodeIndex}`,
          loadedField.values[probe.nodeIndex]?.toFixed(4) ?? "越界",
        ])
      : [];

    // 探针时间序列：已加载时按采样逐行输出。
    const timeSeriesTables = results.probeTimeSeries.map((series) => ({
      probeLabel: `#${series.probeId} · 节点 ${series.nodeIndex}`,
      samples: series.samples.map(
        (sample) => [sample.timeS.toFixed(3), sample.value.toFixed(4)] as [string, string],
      ),
    }));

    const options: ReportOptions = {
      title: template.title,
      notes: template.notes,
      sections: { ...template.sections },
    };
    const html = buildReportHtml(
      {
        projectName: currentProject.name,
        studyName: study.name,
        materialName: material?.name ?? "未登记",
        generatedAt: new Date().toLocaleString("zh-CN", { hour12: false }),
        parameterRows: rows,
        geometryRows,
        snapshots,
        fieldStats,
        probeRows,
        timeSeriesTables,
      },
      options,
    );
    const blob = new Blob([html], { type: "text/html;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `kairos-report-${study.id}.html`;
    anchor.click();
    URL.revokeObjectURL(url);
    status.value = "报告已生成并下载";
  }

  return { status, template, generateReport };
}
