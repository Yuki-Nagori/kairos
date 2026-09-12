/** 工艺状态：成型工艺设置校验问题清单（check_process 命令的域状态）。 */
import { defineStore } from "pinia";
import { checkProcess as apiCheckProcess } from "../api/process";
import type { FillLoadContext } from "../api/process";
import type { ProcessSettings } from "../types";
import { useAppStore } from "./app";

export const useProcessStore = defineStore("process", {
  state: () => ({
    /** 最近一次校验的问题清单（空 = 通过）。 */
    issues: [] as string[],
  }),
  actions: {
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
