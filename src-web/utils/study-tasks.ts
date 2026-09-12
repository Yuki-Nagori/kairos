/**
 * 方案任务序列评估（对齐 Moldflow「方案任务窗格」范式）：任务按必须执行的
 * 顺序排列，每个任务的输出是下一个任务的输入；状态六态 + 阻断——
 * ✓ 成功 / ⚠ 完成但有警告（应检查）/ ✕ 失败 / ⧖ 排队 / ⟳ 执行中 /
 * 空 未开始；上游失败时后续任务进入 blocked（Moldflow 规则：上一任务
 * 失败则不运行后续任务）。纯函数，从应用状态快照评估，可独立单测消融。
 */
import type { GeometrySummary, Job, Material, Project, ResultCatalog } from "../types";
import type { Stage } from "../types";

interface MeshingReportLike {
  elementCount: number;
}

/** 方案任务评估输入：从应用状态裁剪出的最小快照。 */
export interface StudyTasksInput {
  geometries: GeometrySummary[];
  meshReports: Record<string, MeshingReportLike>;
  project: Project | null;
  activeStudyId: string | null;
  materials: { builtin: Material[]; custom: Material[] };
  jobs: Job[];
  resultCatalog: ResultCatalog | null;
}

/** 任务状态：六态对齐 Moldflow 图标语义 + blocked（上游失败阻断）。 */
export type StudyTaskState = "done" | "warning" | "failed" | "queued" | "running" | "todo";

export interface StudyTask {
  id: string;
  label: string;
  state: StudyTaskState | "blocked";
  /** 完成态的摘要信息（如「四面体 5000」「PP-REF-01」）。 */
  detail: string | null;
  /** 未完成时的指引；完成/阻断时为 null（blocked 的说明进 blockReason）。 */
  hint: string | null;
  /** blocked 时的原因（上游任务失败）。 */
  blockReason: string | null;
  /** 双击任务打开的工作台阶段（对齐 Moldflow 双击打开编辑器）。 */
  stage: Stage;
}

/** 几何健康检查是否通过（开放边 / 退化 / 非流形 / 法向不一致任一非零即异常）。 */
function geometryHealthy(geometry: GeometrySummary): boolean {
  const issues = geometry.issues;
  return (
    issues.degenerate === 0 &&
    issues.openEdges === 0 &&
    issues.nonManifoldEdges === 0 &&
    issues.normalInconsistentEdges === 0
  );
}

/** 活跃研究相关作业的任务状态：running > queued > failed > done > 无作业。
 *  作业归属按 studyId 过滤（旧作业 studyId 为 null 时视为公共作业）。 */
function analysisJobState(jobs: Job[], activeStudyId: string | null): StudyTaskState | null {
  const relevant = jobs.filter((job) => job.studyId === null || job.studyId === activeStudyId);
  if (relevant.length === 0) {
    return null;
  }
  if (relevant.some((job) => job.status === "running")) {
    return "running";
  }
  if (relevant.some((job) => job.status === "queued")) {
    return "queued";
  }
  if (relevant.at(-1)?.status === "failed") {
    return "failed";
  }
  if (relevant.some((job) => job.status === "done")) {
    return "done";
  }
  return "todo";
}

/** 评估方案任务序列。序列顺序即执行顺序；输出是下一个任务的输入。 */
export function evaluateStudyTasks(input: StudyTasksInput): StudyTask[] {
  const geometry = input.geometries[0] ?? null;
  const meshReport = geometry !== null ? input.meshReports[geometry.geometryId] : undefined;
  const healthy = geometry !== null && geometryHealthy(geometry);
  const study = input.project?.studies.find((s) => s.id === input.activeStudyId) ?? null;
  const allMaterials = [...input.materials.builtin, ...input.materials.custom];
  const material =
    study?.materialId != null
      ? (allMaterials.find((m) => m.id === study.materialId) ?? null)
      : null;
  const jobState = analysisJobState(input.jobs, input.activeStudyId);
  const analysisBlocked = jobState === "failed";
  const resultBlocked = jobState === "failed" || jobState === "running" || jobState === "queued";
  const resultDone = input.resultCatalog !== null;
  // 一次性求值：时间步数在目录缺失时为 0（?? 两侧均可达，无死分支）
  const resultTimes = input.resultCatalog?.times.length ?? 0;

  const tasks: StudyTask[] = [
    {
      id: "geometry",
      label: "导入几何",
      state: geometry !== null ? "done" : "todo",
      detail: geometry !== null ? `${geometry.fileName}（${geometry.triangleCount} 面）` : null,
      hint: geometry !== null ? null : "在几何面板导入 STL / STEP / IGES 或样例立方体。",
      blockReason: null,
      stage: "geometry",
    },
  ];

  if (geometry !== null) {
    tasks.push({
      id: "mesh",
      label: "网格划分",
      state: meshReport !== undefined ? (healthy ? "done" : "warning") : "todo",
      detail: meshReport !== undefined ? `四面体 ${meshReport.elementCount}` : null,
      hint:
        meshReport !== undefined
          ? healthy
            ? null
            : "网格健康检查有告警，建议执行「修复网格」任务。"
          : "在几何面板设置目标尺寸并生成体积网格。",
      blockReason: null,
      stage: "geometry",
    });
    // 条件任务：几何不健康才出现（对齐 Moldflow 的「网格诊断 / 修复」任务）
    if (!healthy) {
      tasks.push({
        id: "repair",
        label: "修复网格",
        state: "todo",
        detail: null,
        hint: "在几何面板点击「修复」（焊接 / 去退化 / 填孔 / 一致化）。",
        blockReason: null,
        stage: "geometry",
      });
    }
  }

  tasks.push(
    {
      id: "material",
      label: "选择材料",
      state: material !== null ? "done" : "todo",
      detail: material !== null ? material.name : null,
      hint: material !== null ? null : "在材料库面板选择材料并「用于当前研究」。",
      blockReason: null,
      stage: "process",
    },
    {
      id: "process",
      label: "成型工艺设置",
      state: study?.process != null ? "done" : "todo",
      detail:
        study?.process != null
          ? `熔体 ${study.process.meltTempC} °C · 模具 ${study.process.moldTempC} °C`
          : null,
      hint: study?.process != null ? null : "在工艺面板填写参数并「校验并应用到研究」。",
      blockReason: null,
      stage: "process",
    },
    {
      id: "analysis",
      label: "分析（填充 / 保压）",
      state: jobState ?? "todo",
      detail:
        jobState === "done" || jobState === "running" || jobState === "queued"
          ? `${input.jobs.filter((job) => job.studyId === null || job.studyId === input.activeStudyId).length} 个作业`
          : null,
      hint: jobState !== null ? null : "前置就绪后在下方选择分析序列并「提交求解作业」。",
      blockReason: null,
      stage: "solve",
    },
  );

  tasks.push({
    id: "results",
    label: "结果分析",
    state: resultDone ? "done" : resultBlocked ? "blocked" : "todo",
    detail: resultDone ? `${resultTimes} 个时间步` : null,
    hint: resultDone ? null : analysisBlocked ? null : "分析完成后在结果面板扫描 case 目录。",
    blockReason: resultDone
      ? null
      : analysisBlocked
        ? "分析作业失败：检查作业日志与依赖环境，修正后重新提交。"
        : "等待分析完成。",
    stage: "results",
  });

  return tasks;
}

/** 提交前置是否就绪：分析与其后的任务是提交的产物，不参与前置判断。 */
export function prerequisitesReady(tasks: StudyTask[]): boolean {
  const POST_SUBMIT = new Set(["analysis", "results"]);
  return tasks
    .filter((task) => !POST_SUBMIT.has(task.id))
    .every((task) => task.state === "done" || task.state === "warning");
}
