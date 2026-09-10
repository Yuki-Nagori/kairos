/** 流水线前置检查：从应用状态快照纯函数评估五个步骤的就绪情况与下一步指引。 */
import type { GeometrySummary, Job, Material, Project } from "../types";

/** 流水线评估输入：从应用状态裁剪出的最小快照，保持函数纯净可测。 */
export interface PipelineInput {
  geometries: GeometrySummary[];
  meshReports: Record<string, MeshingReportLike>;
  project: Project | null;
  activeStudyId: string | null;
  materials: { builtin: Material[]; custom: Material[] };
  jobs: Job[];
}

interface MeshingReportLike {
  elementCount: number;
}

interface PipelineStep {
  id: string;
  label: string;
  done: boolean;
  /** 未完成时的下一步指引；已完成时为 null。 */
  hint: string | null;
}

/** 填充分析端到端闭环的五个步骤。 */
export function evaluatePipeline(input: PipelineInput): PipelineStep[] {
  const geometry = input.geometries[0] ?? null;
  const meshed = geometry !== null && input.meshReports[geometry.geometryId] !== undefined;
  const study = input.project?.studies.find((s) => s.id === input.activeStudyId) ?? null;
  const allMaterials = [...input.materials.builtin, ...input.materials.custom];
  const material =
    study?.materialId != null
      ? (allMaterials.find((m) => m.id === study.materialId) ?? null)
      : null;
  const processDone = study?.process != null;
  const submitted =
    input.jobs.some((job) => job.status === "running") ||
    input.jobs.some((job) => job.status === "done");

  return [
    {
      id: "geometry",
      label: "导入几何（STL 或样例）",
      done: geometry !== null,
      hint: geometry ? null : "在几何面板点击「导入样例」或导入 STL 文件。",
    },
    {
      id: "mesh",
      label: "生成体积网格",
      done: meshed,
      hint: meshed
        ? null
        : geometry
          ? "在几何面板设置目标尺寸并点击「生成体积网格」。"
          : "先完成几何导入。",
    },
    {
      id: "material",
      label: "选择材料",
      done: material !== null,
      hint: material ? null : "在材料库面板选择材料并点击「用于当前研究」。",
    },
    {
      id: "process",
      label: "设置工艺",
      done: processDone,
      hint: processDone ? null : "在工艺设置面板填写参数并「校验并应用到研究」。",
    },
    {
      id: "submit",
      label: "提交求解作业",
      done: submitted,
      hint: submitted ? null : "全部就绪后点击「提交求解作业」。",
    },
  ];
}

/** 全部前置步骤是否就绪（可提交求解）。 */
export function allPrerequisitesDone(steps: PipelineStep[]): boolean {
  return steps.filter((step) => step.id !== "submit").every((step) => step.done);
}
