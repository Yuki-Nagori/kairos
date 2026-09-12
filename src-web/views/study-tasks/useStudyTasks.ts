/**
 * 方案任务窗格（对齐 Moldflow 方案任务窗格）：左列常驻，任务按执行顺序
 * 排列、六态图标、上游失败阻断后续；双击任务切换到对应工作台阶段编辑。
 * 窗格底部为分析序列选择与提交按钮（替代原流水线面板的编排入口）。
 */
import { computed, ref } from "vue";
import { useAppStore } from "../../stores/app";
import { useGeometryStore } from "../../stores/geometry";
import { useJobsStore } from "../../stores/jobs";
import { useMaterialsStore } from "../../stores/materials";
import { usePipelineStore } from "../../stores/pipeline";
import { useProjectStore } from "../../stores/project";
import { useResultsStore } from "../../stores/results";
import type { AnalysisStage } from "../../types";
import {
  evaluateStudyTasks,
  prerequisitesReady,
  type StudyTask,
  type StudyTaskState,
} from "../../utils/study-tasks";

/** Moldflow 六态图标的展示模型：字符、配色与语义提示。 */
export const TASK_STATE_META: Record<StudyTaskState | "blocked", { icon: string; cls: string }> = {
  done: { icon: "✓", cls: "bg-emerald-800 text-emerald-300" },
  warning: { icon: "!", cls: "bg-amber-500/20 text-amber-400" },
  failed: { icon: "✕", cls: "bg-red-900/60 text-red-400" },
  queued: { icon: "⧖", cls: "border-[1.5px] border-sky-500 text-sky-400" },
  running: { icon: "⟳", cls: "border-[1.5px] border-amber-500 text-amber-400" },
  todo: { icon: "", cls: "border-[1.5px] border-zinc-700" },
  blocked: { icon: "⏸", cls: "border-[1.5px] border-zinc-800 text-zinc-700" },
};

export function useStudyTasks() {
  const app = useAppStore();
  const geometry = useGeometryStore();
  const jobsStore = useJobsStore();
  const materials = useMaterialsStore();
  const pipeline = usePipelineStore();
  const project = useProjectStore();
  const results = useResultsStore();

  // 提交表单：核数留空时按 2 核提交。
  const stage = ref("fill");
  const cores = ref("");

  const tasks = computed<StudyTask[]>(() =>
    evaluateStudyTasks({
      geometries: geometry.geometries,
      meshReports: geometry.meshReports,
      project: project.project,
      activeStudyId: project.activeStudyId,
      materials: materials.materials,
      jobs: jobsStore.jobs,
      resultCatalog: results.resultCatalog,
    }),
  );

  const submitDisabled = computed(() => !prerequisitesReady(tasks.value) || app.busy !== null);

  function openTask(task: StudyTask): void {
    // 双击任务 → 切换到对应阶段（对齐 Moldflow 双击打开编辑器）
    app.stage = task.stage;
  }

  function submit(): void {
    void pipeline.submitPipeline(Number(cores.value) || 2, stage.value as AnalysisStage);
  }

  return { tasks, submitDisabled, openTask, submit, stage, cores };
}
