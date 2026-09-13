/** 工艺状态：成型工艺设置校验问题清单（check_process 命令的域状态）
 *  与最近一次 case 生成的浇口入口口径回显（有效面积，供工况量级校验使用）。 */
import { defineStore } from "pinia";
import { checkProcess as apiCheckProcess } from "../api/process";
import type { FillLoadContext } from "../api/process";
import type { CaseOutcome, ProcessSettings } from "../types";
import { useAppStore } from "./app";

/** case 生成回显 + 归属研究：跨研究时不复用（避免拿旧浇口面积算新研究）。 */
interface CaseInletRecord {
  studyId: string;
  outcome: CaseOutcome;
}

export const useProcessStore = defineStore("process", {
  state: () => ({
    /** 最近一次校验的问题清单（空 = 通过）。 */
    issues: [] as string[],
    /** 最近一次 case 生成的浇口入口回显（null = 尚未生成）。 */
    caseInlet: null as CaseInletRecord | null,
  }),
  actions: {
    /** 记录 case 生成的入口口径（流水线生成 case 后调用）。 */
    recordCaseInlet(studyId: string, outcome: CaseOutcome): void {
      this.caseInlet = { studyId, outcome };
    },
    /** 指定研究的有效浇口面积（m²）：有该研究的 case 回显时用实际值。 */
    effectiveInletAreaM2(studyId: string | null): number | undefined {
      if (studyId === null || this.caseInlet?.studyId !== studyId) {
        return undefined;
      }
      return this.caseInlet.outcome.inletAreaM2 > 0
        ? this.caseInlet.outcome.inletAreaM2
        : undefined;
    },
    /** 校验工艺设置；返回是否通过，问题清单与失败都进状态/全局错误。
     *  `context` 提供件体积/浇口半径时追加填充工况量级检查。 */
    async checkProcess(settings: ProcessSettings, context?: FillLoadContext): Promise<boolean> {
      const app = useAppStore();
      try {
        this.issues = await apiCheckProcess(settings, context);
        return this.issues.length === 0;
      } catch (error) {
        app.setError(error);
        return false;
      }
    },
  },
});
