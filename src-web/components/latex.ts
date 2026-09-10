/**
 * LaTeX 公式渲染（KaTeX）：renderToString 生成 HTML，
 * throwOnError 关闭以保证个别公式错误不炸整个 UI。
 * 颜色随主题：容器继承文字色，KaTeX 默认继承 currentColor。
 */
import katex from "katex";
import "katex/dist/katex.min.css";

/** 渲染一段 LaTeX 公式为 DOM 元素。 */
export function latexBlock(tex: string, displayMode = true): HTMLElement {
  const el = document.createElement("div");
  el.innerHTML = katex.renderToString(tex, { displayMode, throwOnError: false });
  if (displayMode) {
    el.className = "overflow-x-auto text-center text-[11px] text-zinc-300";
  }
  return el;
}
