/**
 * 主题管理器：动态发现 + 注入 + 切换 + 持久化。
 *
 * 主题 = `theme/` 目录下的 CSS 文件名（不含扩展名）。
 * 新增主题 = 丢一个 CSS 文件进来，无需改任何代码。
 * CSS 通过 `[data-theme="名称"]` 选择器定义变量，`<html data-theme="...">` 激活。
 */
import { onUnmounted, ref } from "vue";
import { storageGet, storageKey, storageSet } from "../utils/storage";

/** 主题名 → CSS 原文（构建期从 theme/*.css 提取）；按文件名排序注入，后注入者覆盖先注入者。 */
const THEME_STYLES: [string, string][] = Object.entries(
  import.meta.glob("../theme/*.css", { eager: true, query: "?raw", import: "default" }),
)
  .map(
    ([path, css]) =>
      [path.replace("../theme/", "").replace(".css", ""), css as string] as [string, string],
  )
  .sort(([a], [b]) => a.localeCompare(b));

/** 主题持久化 key（统一网关，值为 JSON 字符串）。 */
const STORAGE_KEY = storageKey("theme", "active");

/** 主题切换后广播的窗口事件：标题栏图标、图表与视口重绘都监听它。 */
export const THEME_CHANGED_EVENT = "kairos:theme-changed";

function resolveSaved(): string {
  const saved = storageGet<string | null>(STORAGE_KEY, null);
  const names = THEME_STYLES.map(([name]) => name);
  return saved !== null && names.includes(saved) ? saved : (names[0] ?? "dark");
}

function apply(theme: string): void {
  document.documentElement.dataset.theme = theme;
}

/** 初始化：把全部主题 CSS 注入 <head>（[data-theme] 选择器天然只激活当前主题），再恢复持久化偏好。 */
export function initTheme(): string {
  for (const [name, css] of THEME_STYLES) {
    const style = document.createElement("style");
    style.dataset.kairosTheme = name;
    style.textContent = css;
    document.head.append(style);
  }
  const theme = resolveSaved();
  apply(theme);
  return theme;
}

export function getActiveTheme(): string {
  return document.documentElement.dataset.theme ?? THEME_STYLES[0]?.[0] ?? "dark";
}

function setTheme(name: string): void {
  apply(name);
  storageSet(STORAGE_KEY, name);
}

/** 循环切换到下一个可用主题。 */
export function cycleTheme(): string {
  const active = getActiveTheme();
  const index = THEME_STYLES.findIndex(([name]) => name === active);
  const next = THEME_STYLES[(index + 1) % THEME_STYLES.length]?.[0] ?? active;
  setTheme(next);
  window.dispatchEvent(new CustomEvent(THEME_CHANGED_EVENT));
  return next;
}

/** 组合式入口：组件内拿到响应式当前主题名；切换广播经事件监听同步，卸载时清理监听。 */
export function useTheme() {
  const theme = ref(getActiveTheme());
  const sync = (): void => {
    theme.value = getActiveTheme();
  };
  window.addEventListener(THEME_CHANGED_EVENT, sync);
  onUnmounted(() => window.removeEventListener(THEME_CHANGED_EVENT, sync));
  return { theme, cycleTheme };
}
