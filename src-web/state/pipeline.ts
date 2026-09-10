import { defaultCaseDir } from "../api/project";
import { generateOpenfoamCase } from "../api/solver";
import type { AnalysisStage } from "../types";
import { appStore, setError } from "./store";
import { submitJobAction } from "./jobs";

/** 端到端提交：case 生成 → 作业入队（前置检查见 T11 流水线面板）。 */
export async function submitPipeline(cores: number, stage: AnalysisStage): Promise<void> {
  const state = appStore.get();
  const geometry = state.geometries[0] ?? null;
  const study = state.project?.studies.find((s) => s.id === state.activeStudyId) ?? null;
  const material = study?.materialId
    ? ([...state.materials.builtin, ...state.materials.custom].find(
        (m) => m.id === study.materialId,
      ) ?? null)
    : null;
  const meshed = geometry !== null && state.meshReports[geometry.geometryId] !== undefined;

  if (geometry === null) {
    setError("请先导入几何。");
    return;
  }
  if (!meshed) {
    setError("请先生成体积网格。");
    return;
  }
  if (material === null) {
    setError("请先在研究上登记材料。");
    return;
  }
  if (study?.process == null) {
    setError("请先设置工艺并应用到研究。");
    return;
  }

  appStore.set({ busy: "正在准备求解…", error: null });
  try {
    const caseDir = await defaultCaseDir(study.id);
    await generateOpenfoamCase({
      geometryId: geometry.geometryId,
      caseDir,
      material,
      process: study.process,
      stage,
      cores,
    });
    await submitJobAction(caseDir, cores);
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}
