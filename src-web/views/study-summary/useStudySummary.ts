/**
 * 方案摘要面板（配置速览 + ui.html 设计稿
 * 右列摘要卡）：材料 / 成型工艺 / 最新作业三段只读属性行，常驻右列首位，
 * 让当前方案的配置在任意阶段一眼可查；编辑仍走各阶段的面板。
 */
import { computed } from "vue";
import { useJobsStore } from "../../stores/jobs";
import { useMaterialsStore } from "../../stores/materials";
import { useProjectStore } from "../../stores/project";
import type { Job } from "../../types";

/** 作业状态的展示文案（与作业面板口径一致）。 */
const JOB_STATUS_LABEL: Record<Job["status"], string> = {
  queued: "排队中",
  running: "运行中",
  done: "已完成",
  failed: "失败",
  cancelled: "已取消",
};

interface SummaryRow {
  k: string;
  v: string;
}

export function useStudySummary() {
  const project = useProjectStore();
  const materials = useMaterialsStore();
  const jobsStore = useJobsStore();

  const study = computed(() => project.activeStudy);

  /** 材料：活跃研究登记的材料在库中的信息。 */
  const materialRows = computed<SummaryRow[] | null>(() => {
    const materialId = study.value?.materialId;
    if (materialId == null) {
      return null;
    }
    const material = [...materials.materials.builtin, ...materials.materials.custom].find(
      (m) => m.id === materialId,
    );
    if (material === undefined) {
      return null;
    }
    return [
      { k: "牌号", v: material.name },
      { k: "家族", v: material.family },
      { k: "供应商", v: material.manufacturer },
    ];
  });

  /** 工艺：活跃研究的成型工艺参数速览。 */
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

  /** 最新作业（不区分研究的最近一笔）：状态与实时物理时间。 */
  const jobRows = computed<SummaryRow[] | null>(() => {
    const job = jobsStore.jobs.at(-1);
    if (job === undefined) {
      return null;
    }
    const rows: SummaryRow[] = [
      { k: "状态", v: JOB_STATUS_LABEL[job.status] },
      { k: "并行核数", v: `${job.cores}` },
    ];
    if (job.status === "running" && job.lastTimeS !== null) {
      rows.push({ k: "物理时间", v: `${job.lastTimeS.toFixed(2)} s` });
    }
    return rows;
  });

  return { materialRows, processRows, jobRows };
}
