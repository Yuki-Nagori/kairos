import { defineStore } from "pinia";
import { defaultCaseDir } from "../api/project";
import { generateOpenfoamCase } from "../api/solver";
import type { AnalysisStage } from "../types";
import { useAppStore } from "./app";
import { useGeometryStore } from "./geometry";
import { useJobsStore } from "./jobs";
import { useMaterialsStore } from "./materials";
import { useProjectStore } from "./project";

/** 流水线编排：跨 store 读取前置条件并端到端提交，无自有状态。 */
export const usePipelineStore = defineStore("pipeline", {
  state: () => ({}),
  actions: {
    /** 端到端提交：case 生成 → 作业入队（前置检查见 T11 流水线面板）。 */
    async submitPipeline(cores: number, stage: AnalysisStage): Promise<void> {
      const app = useAppStore();
      const geometryStore = useGeometryStore();
      const projectStore = useProjectStore();
      const materialsStore = useMaterialsStore();
      const geometry = geometryStore.geometries[0] ?? null;
      const study = projectStore.activeStudy;
      const material = study?.materialId
        ? ([...materialsStore.materials.builtin, ...materialsStore.materials.custom].find(
            (m) => m.id === study.materialId,
          ) ?? null)
        : null;
      const meshed =
        geometry !== null && geometryStore.meshReports[geometry.geometryId] !== undefined;

      if (geometry === null) {
        app.setError("请先导入几何。");
        return;
      }
      if (!meshed) {
        app.setError("请先生成体积网格。");
        return;
      }
      if (material === null) {
        app.setError("请先在研究上登记材料。");
        return;
      }
      if (study?.process == null) {
        app.setError("请先设置工艺并应用到研究。");
        return;
      }

      app.beginBusy("正在准备求解…");
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
        await useJobsStore().submitJob(caseDir, cores);
      } catch (error) {
        app.setError(error);
      } finally {
        app.endBusy();
      }
    },
  },
});
