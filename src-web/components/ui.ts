/** 共享的 DOM 构建辅助，统一 Tailwind 样式基调。 */

export function card(title: string): { root: HTMLElement; body: HTMLElement } {
  const root = document.createElement("section");
  root.className = "rounded-2xl border border-zinc-800 bg-zinc-900 shadow-xl";
  const heading = document.createElement("h2");
  heading.className = "border-b border-zinc-800 px-6 py-4 text-base font-semibold";
  heading.textContent = title;
  const body = document.createElement("div");
  body.className = "space-y-4 px-6 py-5";
  root.append(heading, body);
  return { root, body };
}

export function hint(text: string): HTMLParagraphElement {
  const element = document.createElement("p");
  element.className = "text-sm text-zinc-500";
  element.textContent = text;
  return element;
}
