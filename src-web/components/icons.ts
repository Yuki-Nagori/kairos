/**
 * 内联 SVG 图标：stroke 一律走 currentColor，颜色由外部 className 传入。
 *
 * 来源：Icons8（icons8.com）的终端图标——免费授权要求使用时署名，
 * 已在 README「致谢」标注；不得用作产品 Logo 或单独再分发图标文件。
 *
 * 实现用 createElementNS 逐节点构建（而非 innerHTML 字符串）：
 * WKWebView 下 innerHTML 内联 SVG 存在渲染差异（浏览器正常、Tauri 窗口
 * 不显示，真机踩过），createElementNS 是跨引擎一致的标准做法。
 */
const SVG_NS = "http://www.w3.org/2000/svg";

function node(name: string, attrs: Record<string, string>): SVGElement {
  const el = document.createElementNS(SVG_NS, name);
  for (const [key, value] of Object.entries(attrs)) {
    el.setAttribute(key, value);
  }
  return el;
}

/** 终端窗口图标（shell.svg 的 currentColor 版本）。 */
export function createShellIcon(className = "h-4 w-4"): HTMLElement {
  const span = document.createElement("span");
  span.className = `inline-flex items-center ${className}`;
  span.setAttribute("aria-hidden", "true");

  const svg = node("svg", { viewBox: "0 0 48 48", fill: "none" });
  svg.append(
    node("rect", {
      x: "7.5",
      y: "7.5",
      width: "33",
      height: "33",
      rx: "5",
      stroke: "currentColor",
      "stroke-width": "3",
      "stroke-linejoin": "round",
    }),
    node("line", {
      x1: "40.5",
      y1: "15.5",
      x2: "7.5",
      y2: "15.5",
      stroke: "currentColor",
      "stroke-width": "3",
      "stroke-linejoin": "round",
    }),
    node("polyline", {
      points: "17.5,23.5 21.5,27.5 17.5,31.5",
      stroke: "currentColor",
      "stroke-width": "3",
      fill: "none",
      "stroke-linecap": "round",
      "stroke-linejoin": "round",
    }),
    node("line", {
      x1: "25.5",
      y1: "31.5",
      x2: "31.5",
      y2: "31.5",
      stroke: "currentColor",
      "stroke-width": "3",
      "stroke-linecap": "round",
    }),
  );
  span.append(svg);
  return span;
}
