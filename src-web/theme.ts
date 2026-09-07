/**
 * 主题管理器：动态发现 + 切换 + 持久化。
 *
 * 主题 = `theme/` 目录下的 CSS 文件名（不含扩展名）。
 * 新增主题 = 丢一个 CSS 文件进来，无需改任何代码。
 * CSS 通过 `[data-theme="名称"]` 选择器定义变量，`<html data-theme="...">` 激活。
 */

/** 可用主题名列表（自动从 theme/*.css 文件名提取，按字母排序）。 */
const THEMES: readonly string[] = Object.keys(
  import.meta.glob("./theme/*.css", { eager: true, query: "?raw", import: "default" }),
)
  .map((path) => path.replace("./theme/", "").replace(".css", ""))
  .sort();

const STORAGE_KEY = "kairos-theme";

function resolveSaved(): string {
  const saved = localStorage.getItem(STORAGE_KEY);
  return saved !== null && THEMES.includes(saved) ? saved : (THEMES[0] ?? "dark");
}

function apply(theme: string): void {
  document.documentElement.dataset.theme = theme;
}

/** 初始化：启动时调用一次，恢复持久化的主题偏好。 */
export function initTheme(): string {
  const theme = resolveSaved();
  apply(theme);
  return theme;
}

export function getActiveTheme(): string {
  return document.documentElement.dataset.theme ?? THEMES[0] ?? "dark";
}

function setTheme(name: string): void {
  apply(name);
  localStorage.setItem(STORAGE_KEY, name);
}

/** 循环切换到下一个可用主题。 */
export function cycleTheme(): string {
  const index = THEMES.indexOf(getActiveTheme());
  const next = THEMES[(index + 1) % THEMES.length] ?? THEMES[0] ?? "dark";
  setTheme(next);
  return next;
}
