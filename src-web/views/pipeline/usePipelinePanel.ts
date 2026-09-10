/**
 * 流水线面板：五步闭环的前置检查与下一步指引，引导从几何到求解提交。
 * 提交按钮仅在「前置全部就绪且空闲」时可用；运行中的作业实时显示
 * 物理时间（Job.lastTimeS 由 Rust 侧解析日志得到）。
 */
import { computed, ref } from "vue";
import { useAppStore } from "../../stores/app";
import { useGeometryStore } from "../../stores/geometry";
import { useJobsStore } from "../../stores/jobs";
import { useMaterialsStore } from "../../stores/materials";
import { usePipelineStore } from "../../stores/pipeline";
import { useProjectStore } from "../../stores/project";
import type { AnalysisStage } from "../../types";
import { allPrerequisitesDone, evaluatePipeline } from "../../utils/pipeline";

export function usePipelinePanel() {
  const app = useAppStore();
  const geometry = useGeometryStore();
  const jobsStore = useJobsStore();
  const materials = useMaterialsStore();
  const pipeline = usePipelineStore();
  const project = useProjectStore();

  // 提交表单：核数留空时按 2 核提交。
  const stage = ref("fill");
  const cores = ref("");

  const steps = computed(() =>
    evaluatePipeline({
      geometries: geometry.geometries,
      meshReports: geometry.meshReports,
      project: project.project,
      activeStudyId: project.activeStudyId,
      materials: materials.materials,
      jobs: jobsStore.jobs,
    }),
  );

  const submitDisabled = computed(() => !allPrerequisitesDone(steps.value) || app.busy !== null);

  // 求解实时状态行：显示运行中作业的物理时间，空闲时为空串（行仍占位）。
  const liveText = computed(() => {
    const job = jobsStore.jobs.at(-1);
    if (job?.status === "running" && job.lastTimeS !== null) {
      return `⟳ 求解中 · T = ${job.lastTimeS.toFixed(2)} s`;
    }
    return "";
  });

  function submit(): void {
    void pipeline.submitPipeline(Number(cores.value) || 2, stage.value as AnalysisStage);
  }

  return { stage, cores, steps, submitDisabled, liveText, submit };
}
