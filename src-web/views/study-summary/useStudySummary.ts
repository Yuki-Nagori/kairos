/**
 * 方案摘要面板（配置速览 + ui.html 设计稿
 * 右列摘要卡）：材料 / 成型工艺 / 最新作业三段只读属性行，常驻右列首位，
 * 让当前方案的配置在任意阶段一眼可查；编辑仍走各阶段的面板。
 */
import { computed } from "vue";
import { useMaterialsStore } from "../../stores/materials";
import { useProjectStore } from "../../stores/project";
import { jobStatusLabel, useJobsStore } from "../../stores/jobs";
import { fixed } from "../../utils/format";
import { findMaterial } from "../../utils/materials";

interface SummaryRow {
  k: string;
  v: string;
}

export function useStudySummary() {
  const project = useProjectStore();
  const materials = useMaterialsStore();
  const jobsStore = useJobsStore();

  const study = computed(() => project.activeStudy);

  /** 材料：活跃方案登记的材料在库中的信息。 */
  const materialRows = computed<SummaryRow[] | null>(() => {
    const materialId = study.value?.materialId;
    if (materialId == null) {
      return null;
    }
    const material = findMaterial(materials.materials, materialId);
    if (material === null) {
      return null;
    }
    return [
      { k: "牌号", v: material.name },
      { k: "家族", v: material.family },
      { k: "供应商", v: material.manufacturer },
    ];
  });

  /** 工艺：活跃方案的成型工艺参数速览。 */
  const processRows = computed<SummaryRow[] | null>(() => {
    const process = study.value?.process ?? null;
    if (process === null) {
      return null;
    }
    return [
      { k: "熔体温度", v: `${process.meltTempC} °C` },
      { k: "模具温度", v: `${process.moldTempC} °C` },
      { k: "注射时间", v: `${process.injectionTimeS} s` },
      { k: "V/P 切换", v: `${process.vpSwitchVolumePercent} %` },
      { k: "保压峰值", v: `${process.packingPressureMpaCurve[0]?.[1] ?? 0} MPa` },
      { k: "冷却时间", v: `${process.coolingTimeS} s` },
    ];
  });

  /** 最新作业（不区分方案的最近一笔）：状态与实时物理时间。 */
  const jobRows = computed<SummaryRow[] | null>(() => {
    const job = jobsStore.jobs.at(-1);
    if (job === undefined) {
      return null;
    }
    const rows: SummaryRow[] = [
      { k: "状态", v: jobStatusLabel(job.status) },
      { k: "并行核数", v: `${job.cores}` },
    ];
    if (job.status === "running" && job.lastTimeS !== null) {
      rows.push({ k: "物理时间", v: `${fixed(job.lastTimeS, 2)} s` });
    }
    return rows;
  });

  return { materialRows, processRows, jobRows };
}
