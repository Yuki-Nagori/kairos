<script setup lang="ts">
/** 报告面板：汇总项目/材料/工艺/结果快照，生成自包含 HTML 报告（浏览器可打印 PDF）。 */
import { ref } from "vue";
import { useAppState } from "../../state";
import { buildReportHtml } from "../../lib/report";
import { minMax } from "../../lib/stats";
import { getSnapshotDataUrl } from "../../render/snapshot";
import Card from "../ui/UiCard.vue";
import UiButton from "../ui/UiButton.vue";

const state = useAppState();

// 状态行三种结局：初始引导语 → 缺项目/研究 → 生成成功。
const status = ref("生成自包含 HTML（浏览器打开后 Ctrl+P 打印为 PDF）。");

function generateReport(): void {
  const { project, activeStudyId, materials, loadedField } = state;
  const study = project?.studies.find((s) => s.id === activeStudyId) ?? null;
  if (project === null || study === null) {
    status.value = "请先创建项目与研究。";
    return;
  }
  const material = study.materialId
    ? [...materials.builtin, ...materials.custom].find((m) => m.id === study.materialId)
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
    projectName: project.name,
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
</script>

<template>
  <Card title="仿真报告">
    <UiButton variant="primary" @click="generateReport">生成 HTML 报告</UiButton>
    <p class="text-xs text-zinc-500">{{ status }}</p>
  </Card>
</template>
