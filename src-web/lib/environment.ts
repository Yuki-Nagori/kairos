/** 当前是否运行在 Tauri WebView 内（而非普通浏览器）。 */
export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
