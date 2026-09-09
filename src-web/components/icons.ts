/**
 * 内联 SVG 图标：stroke 一律走 currentColor，颜色由外部 className 传入。
 * 图形源自用户提供的 shell.svg（终端窗口 + 提示符），转 currentColor 以适配主题。
 */

const SHELL_SVG = `<svg viewBox="0 0 48 48" fill="none" xmlns="http://www.w3.org/2000/svg">
  <rect x="7.5" y="7.5" width="33" height="33" rx="5" stroke="currentColor" stroke-width="3" stroke-linejoin="round"></rect>
  <line x1="40.5" y1="15.5" x2="7.5" y2="15.5" stroke="currentColor" stroke-width="3" stroke-linejoin="round"></line>
  <polyline points="17.5,23.5 21.5,27.5 17.5,31.5" stroke="currentColor" stroke-width="3" fill="none" stroke-linecap="round" stroke-linejoin="round"></polyline>
  <line x1="25.5" y1="31.5" x2="31.5" y2="31.5" stroke="currentColor" stroke-width="3" stroke-linecap="round"></line>
</svg>`;

/** 终端窗口图标（shell.svg 的 currentColor 版本）。 */
export function createShellIcon(className = "h-4 w-4"): HTMLElement {
  const span = document.createElement("span");
  span.className = `inline-flex items-center ${className}`;
  span.setAttribute("aria-hidden", "true");
  span.innerHTML = SHELL_SVG;
  return span;
}
