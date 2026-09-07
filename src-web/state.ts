import { createStore } from "./lib/store";
import { IpcUnavailableError } from "./lib/ipc";
import { getSystemInfo } from "./services/system";
import type { SystemInfo } from "./types";

/** 全局应用状态：组件经 select/subscribe 订阅，只能通过本文件的动作函数修改。 */
interface AppState {
  /** 应用信息，bootstrap 成功后填充；非 null 即代表 IPC 链路通畅。 */
  info: SystemInfo | null;
  /** 进行中的异步操作提示文案，标题栏展示；null 表示空闲。 */
  busy: string | null;
  /** 最近一次错误；info 为环境提示（浏览器预览，自动消失），否则是真实失败。 */
  error: { message: string; info: boolean } | null;
}

export const initialAppState: AppState = {
  info: null,
  busy: null,
  error: null,
};

export const appStore = createStore<AppState>(initialAppState);

let errorTimer: ReturnType<typeof setTimeout> | undefined;

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** 统一的错误入口：IPC 不可用是预期内的环境提示（自动消失），其余才是真实失败。 */
export function setError(error: unknown): void {
  const message = toMessage(error);
  const info = error instanceof IpcUnavailableError;
  if (errorTimer !== undefined) {
    clearTimeout(errorTimer);
  }
  appStore.set({ error: { message, info } });
  if (info) {
    errorTimer = setTimeout(() => {
      if (appStore.get().error?.message === message) {
        appStore.set({ error: null });
      }
    }, 8000);
  }
}

/** 启动时拉取应用信息，顺带验证 IPC 链路是否通畅。 */
export async function bootstrap(): Promise<void> {
  try {
    const info = await getSystemInfo();
    appStore.set({ info });
  } catch (error) {
    setError(error);
  }
}
