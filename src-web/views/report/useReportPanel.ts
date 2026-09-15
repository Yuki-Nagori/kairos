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
import type { ReportSlidePayload } from "../../api/project";
import { downloadTextFile } from "../../utils/download";
import { fixed, significant } from "../../utils/format";
import { findMaterial } from "../../utils/materials";
import { geometryHealthy } from "../../utils/study-tasks";

export function useReportPanel() {
  const project = useProjectStore();
  const materials = useMaterialsStore();
  const results = useResultsStore();
  const geometry = useGeometryStore();

  // 状态行三种结局：初始引导语 → 缺项目/方案 → 生成成功。
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

  /** 散装工程回退：浏览器下载（工作区不可用时的保底通路）。 */
  function downloadReport(studyId: string, html: string): void {
    downloadTextFile(`kairos-report-${studyId}.html`, html, "text/html;charset=utf-8");
  }

  async function generateReport(): Promise<void> {
    const currentProject = project.project;
    const materialsLib = materials.materials;
    const loadedField = results.loadedField;
    const study = project.activeStudy;
    if (currentProject === null || study === null) {
      status.value = "请先创建项目与方案。";
      return;
    }
    const material = findMaterial(materialsLib, study.materialId);
    const rows: Array<[string, string]> = [
      ["材料", material !== null ? `${material.manufacturer} · ${material.name}` : "未登记"],
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
      fieldStats = `${loadedField.field} @ ${loadedField.timeDir}s：${loadedField.values.length} 个值，min ${significant(min)} / max ${significant(max)}${loadedField.complete ? "" : "（不完整）"}`;
    }

    // 几何摘要：首个已导入几何的规模与健康度；体积网格报告存在时附质量行。
    const geometryRows: Array<[string, string]> = [];
    const geometrySummary = geometry.geometries[0];
    if (geometrySummary !== undefined) {
      const issues = geometrySummary.issues;
      geometryRows.push(
        ["三角形数", String(geometrySummary.triangleCount)],
        [
          "尺寸",
          `${geometrySummary.size.map((value) => fixed(value, 2)).join(" × ")} ${geometrySummary.suggestedUnit}`,
        ],
        [
          "网格健康",
          geometryHealthy(geometrySummary)
            ? "健康"
            : `退化 ${issues.degenerate} / 开放边 ${issues.openEdges} / 非流形 ${issues.nonManifoldEdges}`,
        ],
      );
    }
    const meshReport = geometry.meshReports[geometrySummary?.geometryId ?? ""];
    if (meshReport !== undefined) {
      geometryRows.push([
        "体积网格",
        `${meshReport.engine} · 节点 ${meshReport.nodeCount} · 四面体 ${meshReport.elementCount} · 体积 ${significant(meshReport.totalVolume)}`,
      ]);
      geometryRows.push([
        "网格质量",
        `最长边/最短边 max ${fixed(meshReport.quality.maxEdgeRatio, 2)} · 纵横比 avg ${fixed(meshReport.aspectAvg, 2)} / max ${fixed(meshReport.aspectMax, 2)}`,
      ]);
    }

    // 探针数值：当前加载场下各探针的取值。
    const probeRows: Array<[string, string]> = loadedField
      ? results.probes.map((probe) => [
          `#${probe.id} · 节点 ${probe.nodeIndex}`,
          fixed(loadedField.values[probe.nodeIndex] ?? Number.NaN, 4, "越界"),
        ])
      : [];

    // 探针时间序列：已加载时按采样逐行输出。
    const timeSeriesTables = results.probeTimeSeries.map((series) => ({
      probeLabel: `#${series.probeId} · 节点 ${series.nodeIndex}`,
      samples: series.samples.map(
        (sample) => [significant(sample.timeS), fixed(sample.value, 4)] as [string, string],
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
    const fileName = `kairos-report-${study.name}.html`;
    // 工作区工程：默认写进 <工作区>/reports/（自包含，随工程拷走）；
    // 散装工程或写入失败（目录只读等）回退浏览器下载，保证报告一定能拿到。
    const savedPath = await project.saveReport(fileName, html);
    if (savedPath !== null) {
      status.value = `报告已保存：${savedPath}`;
      return;
    }
    downloadReport(study.id, html);
    status.value = "报告已生成并下载";
  }

  /** 导出 PPTX：与 HTML 报告同源数据，按「工况参数 / 网格与结果」组装要点。 */
  async function exportPptx(): Promise<void> {
    const currentProject = project.project;
    const study = project.activeStudy;
    if (currentProject === null || study === null || project.projectPath === null) {
      status.value = "请先创建项目与方案（散装工程请先保存到工作区）。";
      return;
    }
    const process = study.process;
    const slides: ReportSlidePayload[] = [
      {
        title: "工况参数",
        bullets: [
          `材料：${study.materialId ?? "未登记"}`,
          `熔体温度：${process ? `${process.meltTempC} °C` : "未设置"}`,
          `模具温度：${process ? `${process.moldTempC} °C` : "未设置"}`,
          `注射 / 保压 / 冷却：${process ? `${process.injectionTimeS} / ${process.packingTimeS} / ${process.coolingTimeS} s` : "未设置"}`,
        ],
      },
    ];
    const summary = geometry.geometries[0];
    if (summary !== undefined) {
      slides.push({
        title: "网格与结果",
        bullets: [
          `几何：${summary.size.map((value) => fixed(value, 2)).join(" × ")} ${summary.suggestedUnit}`,
          results.loadedField !== null
            ? `已加载场：${results.loadedField.field} @ ${results.loadedField.timeDir}`
            : "尚未加载结果场",
        ],
      });
    }
    // 快照：与 HTML 报告同一来源（视口 / XY 曲线），有一张就补一页「快照」
    const snapshots: string[] = [];
    const viewport = getSnapshotDataUrl("viewport");
    if (viewport !== null) {
      snapshots.push(viewport);
    }
    const chart = getSnapshotDataUrl("xy-chart");
    if (chart !== null) {
      snapshots.push(chart);
    }
    const labels: string[] = [];
    if (viewport !== null) {
      labels.push("视口");
    }
    if (chart !== null) {
      labels.push("XY 曲线");
    }
    if (labels.length > 0) {
      slides.push({ title: "快照", bullets: labels, images: snapshots });
    }
    const path = await project.saveReportPptx(
      template.title.trim() || `${currentProject.name} 仿真报告`,
      slides,
    );
    if (path !== null) {
      status.value = `已导出 PPTX：${path}`;
    }
  }

  return { status, template, generateReport, exportPptx };
}
