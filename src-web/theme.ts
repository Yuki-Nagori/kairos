/** 主题管理器：深色 / 浅色切换 + localStorage 持久化。 */

type Theme = "dark" | "light";

const STORAGE_KEY = "kairos-theme";

/** 初始化主题：读 localStorage，写 data-theme 到 <html>。 */
export function initTheme(): Theme {
  const saved = localStorage.getItem(STORAGE_KEY);
  const theme: Theme = saved === "light" ? "light" : "dark";
  applyTheme(theme);
  return theme;
}

/** 切换主题。 */
export function toggleTheme(): Theme {
  const current = document.documentElement.dataset.theme === "light" ? "light" : "dark";
  const next: Theme = current === "dark" ? "light" : "dark";
  applyTheme(next);
  localStorage.setItem(STORAGE_KEY, next);
  return next;
}

function applyTheme(theme: Theme): void {
  document.documentElement.dataset.theme = theme;
}
