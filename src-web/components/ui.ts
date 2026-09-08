/**
 * 共享 DOM 构建辅助：面板控件的样式集中此处，避免类名字符串跨文件复制。
 * 主题切换基于 Tailwind v4 的 CSS 变量色板（theme/*.css 覆盖 --color-zinc-*）。
 */

export function card(
  title: string,
  options?: { collapsible?: boolean },
): { root: HTMLElement; body: HTMLElement } {
  const root = document.createElement("section");
  root.className = "rounded-2xl border border-zinc-800 bg-zinc-900 shadow-xl";
  const heading = document.createElement("h2");
  heading.className =
    "border-b border-zinc-800 px-5 py-3 text-sm font-semibold text-zinc-100 cursor-pointer select-none";
  heading.textContent = title;
  const body = document.createElement("div");
  body.className = "space-y-3 px-5 py-4";

  if (options?.collapsible === true) {
    const storageKey = `kairos-panel:${title}`;
    const collapsed = localStorage.getItem(storageKey) === "1";
    body.classList.toggle("hidden", collapsed);
    heading.textContent = `${collapsed ? "\u25b8" : "\u25be"} ${title}`;
    heading.addEventListener("click", () => {
      const nowCollapsed = !body.classList.contains("hidden");
      body.classList.toggle("hidden", nowCollapsed);
      heading.textContent = `${nowCollapsed ? "\u25b8" : "\u25be"} ${title}`;
      localStorage.setItem(storageKey, nowCollapsed ? "1" : "0");
    });
  }

  root.append(heading, body);
  return { root, body };
}

export function hint(text: string): HTMLParagraphElement {
  const element = document.createElement("p");
  element.className = "text-sm text-zinc-500";
  element.textContent = text;
  return element;
}

export function button(
  label: string,
  variant: "primary" | "ghost" | "danger" = "ghost",
): HTMLButtonElement {
  const element = document.createElement("button");
  element.type = "button";
  element.textContent = label;
  element.className = [
    "rounded-lg px-3 py-1.5 text-xs font-medium transition-colors",
    variant === "primary"
      ? "bg-emerald-500 text-emerald-950 hover:bg-emerald-400"
      : variant === "danger"
        ? "border border-zinc-700 text-zinc-400 hover:border-red-500 hover:text-red-400"
        : "border border-zinc-700 text-zinc-200 hover:border-emerald-500 hover:text-emerald-400",
  ].join(" ");
  return element;
}

export function textInput(placeholder: string): HTMLInputElement {
  const element = document.createElement("input");
  element.type = "text";
  element.placeholder = placeholder;
  element.className =
    "rounded-lg border border-zinc-700 bg-zinc-950 px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-emerald-500 focus:outline-none";
  return element;
}

/** 面板内紧凑下拉框；选项由调用方按需填充（含动态刷新场景）。 */
export function dropdown(): HTMLSelectElement {
  const element = document.createElement("select");
  element.className =
    "rounded-lg border border-zinc-700 bg-zinc-950 px-2 py-2 text-xs text-zinc-300 focus:border-emerald-500 focus:outline-none";
  return element;
}
