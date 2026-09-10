/** IPC 网关：统一运行时探测与错误归类，前端只捕获 IpcUnavailableError / CommandError。 */
import { invoke } from "@tauri-apps/api/core";
import { isTauriRuntime } from "./environment";

/** 浏览器预览（无 Tauri 运行时）时抛出：属于预期内的环境提示，而非故障。 */
export class IpcUnavailableError extends Error {
  constructor() {
    super("Not running inside the Tauri runtime — start the app with `bun run tauri dev`.");
    this.name = "IpcUnavailableError";
  }
}

/** Rust 侧 KairosError 的 IPC 形态（契约见 kairos-core/src/error.rs），按 code 分类处理。 */
export class CommandError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "CommandError";
    this.code = code;
  }
}

function normalize(rejection: unknown): Error {
  if (rejection instanceof Error) {
    return rejection;
  }
  if (typeof rejection === "object" && rejection !== null) {
    const shape = rejection as { code?: unknown; message?: unknown };
    if (typeof shape.code === "string" && typeof shape.message === "string") {
      // 命中 KairosError 的 IPC 形态（kairos-core/src/error.rs）
      return new CommandError(shape.code, shape.message);
    }
    if (typeof shape.message === "string") {
      return new Error(shape.message);
    }
    // IPC 传来的都是可结构化克隆的 JSON，不会是循环引用
    return new Error(JSON.stringify(rejection));
  }
  return new Error(String(rejection));
}

/** 全项目唯一的命令调用入口。 */
export async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauriRuntime()) {
    throw new IpcUnavailableError();
  }
  try {
    return await invoke<T>(command, args);
  } catch (rejection) {
    throw normalize(rejection);
  }
}
