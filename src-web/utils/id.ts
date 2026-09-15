/** 生成可持久化的前端 ID；优先使用平台 CSPRNG，旧 WebView 用单调序列兜底。 */
let fallbackSequence = 0;

export function newId(prefix: string): string {
  const randomUuid = globalThis.crypto?.randomUUID;
  const suffix =
    typeof randomUuid === "function"
      ? randomUuid.call(globalThis.crypto)
      : `${Date.now().toString(36)}-${(++fallbackSequence).toString(36)}`;
  return `${prefix}-${suffix}`;
}
