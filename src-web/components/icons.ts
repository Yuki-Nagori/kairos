/**
 * 图片方式加载图标资源（public/shell.svg）。
 *
 * 来源：Icons8（icons8.com）的终端图标——免费授权要求使用时署名，
 * 已在 README「致谢」标注；不得用作产品 Logo 或单独再分发图标文件。
 *
 * 注意：`<img>` 引用的 SVG 无法继承 currentColor，图标颜色固定在
 * 文件内（emerald 绿，深浅主题均可见）；需换色直接编辑该文件。
 */

/** 终端窗口图标（shell.svg）。 */
export function createShellIcon(className = "h-4 w-4"): HTMLElement {
  const img = document.createElement("img");
  img.src = "/shell.svg";
  img.alt = "";
  img.setAttribute("aria-hidden", "true");
  img.className = `inline-flex items-center ${className}`;
  return img;
}
