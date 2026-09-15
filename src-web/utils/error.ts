/**
 * 错误归一化：把任意抛出物取成可展示的一句话。
 *
 * IPC 的拒绝值形态不固定（Error、KairosError 的 `{code,message}` 对象、纯字符串），
 * 统一走这里，避免各 store / composable 各写一遍 `instanceof` 判断而口径漂移。
 * 按 code 分支处置的那一层在 utils/ipc（CommandError），这里只负责取文本。
 */
export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
