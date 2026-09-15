/** 流水线编排（Pinia）：跨 store 读取前置条件并端到端提交，无自有状态。 */
import { defineStore } from "pinia";
import { defaultCaseDir } from "../api/project";
import { generateMoldingfoamCase } from "../api/solver";
import type { AnalysisStage } from "../types";
import { findMaterial } from "../utils/materials";
import { useAppStore } from "./app";
import { useGeometryStore } from "./geometry";
import { useJobsStore } from "./jobs";
import { useMaterialsStore } from "./materials";
import { useProcessStore } from "./process";
import { useProjectStore } from "./project";

export const usePipelineStore = defineStore("pipeline", {
  state: () => ({}),
  actions: {
    /** 端到端提交：case 生成 → 作业入队；前置就绪提示由流水线面板的
     * evaluatePipeline 负责，这里仍做防御校验（不满足直接报错返回）。 */
    async submitPipeline(cores: number, stage: AnalysisStage): Promise<void> {
      const app = useAppStore();
      const geometryStore = useGeometryStore();
      const projectStore = useProjectStore();
      const materialsStore = useMaterialsStore();
      const geometry = geometryStore.geometries[0] ?? null;
      const study = projectStore.activeStudy;
      const material = findMaterial(materialsStore.materials, study?.materialId ?? null);
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
        app.setError("请先在方案上登记材料。");
        return;
      }
      const activeStudy = study;
      const process = activeStudy?.process ?? null;
      if (activeStudy === null || process === null) {
        app.setError("请先设置工艺并应用到方案。");
        return;
      }

      await app.withBusy("正在准备求解…", async () => {
        // case 目录：工作区工程放 <工作区>/cases/<方案 id>，散装工程回退应用数据目录
        const caseDir = await defaultCaseDir(activeStudy.id, projectStore.projectPath);
        const outcome = await generateMoldingfoamCase({
          geometryId: geometry.geometryId,
          caseDir,
          material,
          process,
          stage,
          cores,
          runnerElements: activeStudy.runnerElements,
          coolingChannels: activeStudy.coolingChannels,
        });
        // 入口口径回显落到工艺 store：工艺面板的工况量级校验改用有效面积。
        useProcessStore().recordCaseInlet(activeStudy.id, outcome);
        await useJobsStore().submitJob(caseDir, cores, activeStudy.id);
      });
    },
  },
});
