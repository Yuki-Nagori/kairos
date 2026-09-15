/**
 * 应用级全局状态：版本/IPC 信息、分析阶段、忙碌提示与全局错误。
 * 错误分两类——IPC 不可用是预期内的环境提示（自动消失），其余是真实失败；
 * 统一经 setError 进入，状态栏按「错误 > 忙碌 > 版本」优先级展示。
 * CommandError 的 code 随错误保留（契约见 kairos-core/src/error.rs），
 * 状态栏按 code 给出处置提示，禁止对 message 做文本匹配。
 */
import { defineStore } from "pinia";
import { errorMessage } from "../utils/error";
import { CommandError, IpcUnavailableError } from "../utils/ipc";
import type { Stage, SystemInfo } from "../types";

let errorTimer: ReturnType<typeof setTimeout> | undefined;

export const useAppStore = defineStore("app", {
  state: () => ({
    info: null as SystemInfo | null,
    stage: "home" as Stage,
    busy: null as string | null,
    busyTasks: [] as { id: number; label: string }[],
    busySequence: 0,
    error: null as { message: string; info: boolean; code: string | null } | null,
  }),
  getters: {
    /** 「有动作在进行中」的唯一判据：各面板的禁用态都读它，不各自写 busy !== null。 */
    working: (state): boolean => state.busy !== null,
  },
  actions: {
    setError(error: unknown): void {
      const message = errorMessage(error);
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
    beginBusy(label: string): number {
      const id = ++this.busySequence;
      this.busyTasks.push({ id, label });
      this.busy = label;
      this.error = null;
      return id;
    },
    /** 按操作身份移除忙碌状态，允许嵌套与并发操作以任意顺序结束。 */
    endBusy(id?: number): void {
      const completed = id ?? this.busyTasks.at(-1)?.id;
      this.busyTasks = this.busyTasks.filter((task) => task.id !== completed);
      this.busy = this.busyTasks.at(-1)?.label ?? null;
    },
    /** 领域动作统一包装：busy 提示 → 执行 → 失败进全局错误 → 复位 busy。
     *  各 store 的异步动作都经此编排，避免逐处重复 try/catch/finally 样板；
     *  返回值透传，失败返回 undefined（错误已入全局状态）。 */
    async withBusy<T>(label: string, run: () => Promise<T>): Promise<T | undefined> {
      const id = this.beginBusy(label);
      try {
        return await run();
      } catch (error) {
        this.setError(error);
        return undefined;
      } finally {
        this.endBusy(id);
      }
    },
  },
});
