/**
 * 主题管理器：动态发现 + 注入 + 切换 + 持久化。
 *
 * 主题 = `theme/` 目录下的 CSS 文件名（不含扩展名）。
 * 新增主题 = 丢一个 CSS 文件进来，无需改任何代码。
 * CSS 通过 `[data-theme="名称"]` 选择器定义变量，`<html data-theme="...">` 激活。
 */

/** 主题名 → CSS 原文（构建期从 theme/*.css 提取）；按文件名排序注入，后注入者覆盖先注入者。 */
const THEME_STYLES: [string, string][] = Object.entries(
  import.meta.glob("./theme/*.css", { eager: true, query: "?raw", import: "default" }),
)
  .map(
    ([path, css]) =>
      [path.replace("./theme/", "").replace(".css", ""), css as string] as [string, string],
  )
  .sort(([a], [b]) => a.localeCompare(b));

const STORAGE_KEY = "kairos-theme";

function resolveSaved(): string {
  const saved = localStorage.getItem(STORAGE_KEY);
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
  localStorage.setItem(STORAGE_KEY, name);
}

/** 循环切换到下一个可用主题。 */
export function cycleTheme(): string {
  const active = getActiveTheme();
  const index = THEME_STYLES.findIndex(([name]) => name === active);
  const next = THEME_STYLES[(index + 1) % THEME_STYLES.length]?.[0] ?? active;
  setTheme(next);
  return next;
}
