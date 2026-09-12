/**
 * 方案任务窗格（对齐 Moldflow 方案任务窗格）：左列常驻，任务按执行顺序
 * 排列、六态图标、上游失败阻断后续；双击任务切换到对应工作台阶段编辑，
 * 右键任务弹出常用命令菜单（打开编辑 / 取消作业 / 重扫结果目录）。
 * 窗格底部为分析序列选择与提交按钮（替代原流水线面板的编排入口）。
 * 任务序列本身来自共享快照 composables/useStudyTasksSnapshot（与阶段选项
 * 卡角标共用），本 composable 只补窗格自身的交互与提交表单。
 */
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useAppStore } from "../../stores/app";
import { useJobsStore } from "../../stores/jobs";
import { usePipelineStore } from "../../stores/pipeline";
import { useProjectStore } from "../../stores/project";
import { useResultsStore } from "../../stores/results";
import type { AnalysisStage } from "../../types";
import { useStudyTasksSnapshot } from "../../composables/useStudyTasksSnapshot";
import { prerequisitesReady, type StudyTask, type StudyTaskState } from "../../utils/study-tasks";

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

interface TaskMenuItem {
  label: string;
  danger?: boolean;
  run: () => void;
}

export function useStudyTasks() {
  const app = useAppStore();
  const jobsStore = useJobsStore();
  const pipeline = usePipelineStore();
  const project = useProjectStore();
  const results = useResultsStore();
  const { tasks } = useStudyTasksSnapshot();

  // 提交表单：核数留空时按 2 核提交。
  const stage = ref("fill");
  const cores = ref("");

  const submitDisabled = computed(() => !prerequisitesReady(tasks.value) || app.busy !== null);

  function openTask(task: StudyTask): void {
    // 双击任务 → 切换到对应阶段（对齐 Moldflow 双击打开编辑器）
    app.stage = task.stage;
  }

  function submit(): void {
    void pipeline.submitPipeline(Number(cores.value) || 2, stage.value as AnalysisStage);
  }

  // —— 右键菜单（对齐 Moldflow 任务右键常用命令）——
  const menu = ref<{ task: StudyTask; x: number; y: number } | null>(null);

  function openMenu(task: StudyTask, event: MouseEvent): void {
    // 视口右缘 / 下缘夹取，避免菜单溢出屏幕
    const x = Math.min(event.clientX, window.innerWidth - 180);
    const y = Math.min(event.clientY, window.innerHeight - 120);
    menu.value = { task, x, y };
  }

  function closeMenu(): void {
    menu.value = null;
  }

  /** 菜单任务的常用命令：打开编辑恒有；取消作业（运行 / 排队中）、
   *  重扫结果目录（已扫描过）按任务状态出现。由模板在菜单打开时调用。 */
  function taskMenuCommands(task: StudyTask): TaskMenuItem[] {
    const items: TaskMenuItem[] = [{ label: "打开编辑", run: () => openTask(task) }];
    if (task.id === "analysis" && (task.state === "running" || task.state === "queued")) {
      // 不变量：运行 / 排队态由相关作业派生，必有作业可取消
      const job = jobsStore.jobs
        .filter((j) => j.studyId === null || j.studyId === project.activeStudyId)
        .at(-1)!;
      items.push({
        label: "取消作业",
        danger: true,
        run: () => void jobsStore.cancelJob(job.id),
      });
    }
    if (task.id === "results" && results.resultCatalog !== null) {
      items.push({ label: "重扫结果目录", run: () => void results.rescanCatalog() });
    }
    return items;
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      closeMenu();
    }
  }
  onMounted(() => {
    window.addEventListener("click", closeMenu);
    window.addEventListener("keydown", onKeydown);
  });
  onUnmounted(() => {
    window.removeEventListener("click", closeMenu);
    window.removeEventListener("keydown", onKeydown);
  });

  return {
    tasks,
    submitDisabled,
    openTask,
    submit,
    stage,
    cores,
    menu,
    openMenu,
    closeMenu,
    taskMenuCommands,
  };
}
