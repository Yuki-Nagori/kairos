/** 报告面板：汇总项目/材料/工艺/结果快照，生成自包含 HTML 报告（浏览器可打印 PDF）。 */
import { ref } from "vue";
import { useMaterialsStore } from "../../stores/materials";
import { useProjectStore } from "../../stores/project";
import { useResultsStore } from "../../stores/results";
import { buildReportHtml } from "../../utils/report";
import { minMax } from "../../utils/stats";
import { getSnapshotDataUrl } from "../../render/snapshot";

export function useReportPanel() {
  const project = useProjectStore();
  const materials = useMaterialsStore();
  const results = useResultsStore();

  // 状态行三种结局：初始引导语 → 缺项目/研究 → 生成成功。
  const status = ref("生成自包含 HTML（浏览器打开后 Ctrl+P 打印为 PDF）。");

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
    const html = buildReportHtml({
      projectName: currentProject.name,
      studyName: study.name,
      materialName: material?.name ?? "未登记",
      generatedAt: new Date().toLocaleString("zh-CN", { hour12: false }),
      parameterRows: rows,
      snapshots,
      fieldStats,
    });
    const blob = new Blob([html], { type: "text/html;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `kairos-report-${study.id}.html`;
    anchor.click();
    URL.revokeObjectURL(url);
    status.value = "报告已生成并下载";
  }

  return { status, generateReport };
}
