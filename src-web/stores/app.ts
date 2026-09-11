/**
 * 应用级全局状态：版本/IPC 信息、分析阶段、忙碌提示与全局错误。
 * 错误分两类——IPC 不可用是预期内的环境提示（自动消失），其余是真实失败；
 * 统一经 setError 进入，状态栏按「错误 > 忙碌 > 版本」优先级展示。
 * CommandError 的 code 随错误保留（契约见 kairos-core/src/error.rs），
 * 状态栏按 code 给出处置提示，禁止对 message 做文本匹配。
 */
import { defineStore } from "pinia";
import { CommandError, IpcUnavailableError } from "../utils/ipc";
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
    error: null as { message: string; info: boolean; code: string | null } | null,
  }),
  actions: {
    setError(error: unknown): void {
      const message = toMessage(error);
      const info = error instanceof IpcUnavailableError;
      const code = error instanceof CommandError ? error.code : null;
      if (errorTimer !== undefined) {
        clearTimeout(errorTimer);
      }
      this.error = { message, info, code };
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
    /** 领域动作统一包装：busy 提示 → 执行 → 失败进全局错误 → 复位 busy。
     *  各 store 的异步动作都经此编排，避免逐处重复 try/catch/finally 样板；
     *  返回值透传，失败返回 undefined（错误已入全局状态）。 */
    async withBusy<T>(label: string, run: () => Promise<T>): Promise<T | undefined> {
      this.beginBusy(label);
      try {
        return await run();
      } catch (error) {
        this.setError(error);
        return undefined;
      } finally {
        this.endBusy();
      }
    },
  },
});
