/**
 * 应用级全局状态：版本/IPC 信息、分析阶段、忙碌提示与全局错误。
 * 错误分两类——IPC 不可用是预期内的环境提示（自动消失），其余是真实失败；
 * 统一经 setError 进入，状态栏按「错误 > 忙碌 > 版本」优先级展示。
 */
import { defineStore } from "pinia";
import { IpcUnavailableError } from "../utils/ipc";
import type { Stage, SystemInfo } from "../types";

let errorTimer: ReturnType<typeof setTimeout> | undefined;

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export const useAppStore = defineStore("app", {
  state: () => ({
    info: null as SystemInfo | null,
    stage: "home" as Stage,
    busy: null as string | null,
    error: null as { message: string; info: boolean } | null,
  }),
  actions: {
    setError(error: unknown): void {
      const message = toMessage(error);
      const info = error instanceof IpcUnavailableError;
      if (errorTimer !== undefined) {
        clearTimeout(errorTimer);
      }
      this.error = { message, info };
      if (info) {
        errorTimer = setTimeout(() => {
          if (this.error?.message === message) {
            this.error = null;
          }
        }, 8000);
      }
    },
    /** 领域动作统一的前置清理：进入忙碌态并清掉上次错误。 */
    beginBusy(label: string): void {
      this.busy = label;
      this.error = null;
    },
    endBusy(): void {
      this.busy = null;
    },
  },
});
