/**
 * 共享 DOM 构建辅助：面板控件的样式集中此处，避免类名字符串跨文件复制。
 * 主题切换基于 Tailwind v4 的 CSS 变量色板（theme/*.css 覆盖 --color-zinc-*）。
 */

/** 面板卡片：着色头部条 + 内容区；collapsible 时点击头部折叠（状态入 localStorage）。 */
export function card(
  title: string,
  options?: { collapsible?: boolean },
): { root: HTMLElement; body: HTMLElement } {
  const root = document.createElement("section");
  root.className =
    "overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900 shadow-lg shadow-black/20";

  const heading = document.createElement("h2");
  heading.className =
    "flex items-center gap-2 border-b border-zinc-800 bg-zinc-950/40 px-4 py-2.5 text-xs font-semibold tracking-wide text-zinc-200 select-none";
  heading.textContent = title;

  const body = document.createElement("div");
  body.className = "space-y-3 px-4 py-3.5";

  if (options?.collapsible === true) {
    const storageKey = `kairos-panel:${title}`;
    const collapsed = localStorage.getItem(storageKey) === "1";
    body.classList.toggle("hidden", collapsed);
    heading.classList.toggle("cursor-pointer");
    const chevron = document.createElement("span");
    chevron.className = "ml-auto text-zinc-500";
    chevron.textContent = collapsed ? "\u25b8" : "\u25be";
    heading.append(chevron);
    heading.addEventListener("click", () => {
      const nowCollapsed = !body.classList.contains("hidden");
      body.classList.toggle("hidden", nowCollapsed);
      chevron.textContent = nowCollapsed ? "\u25b8" : "\u25be";
      localStorage.setItem(storageKey, nowCollapsed ? "1" : "0");
    });
  }

  root.append(heading, body);
  return { root, body };
}

/** 弱提示行。 */
export function hint(text: string): HTMLParagraphElement {
  const element = document.createElement("p");
  element.className = "text-xs text-zinc-500";
  element.textContent = text;
  return element;
}

/** 小节标签：表单分组的小标题。 */
export function sectionLabel(text: string): HTMLParagraphElement {
  const element = document.createElement("p");
  element.className = "text-[11px] font-semibold tracking-wide text-zinc-400";
  element.textContent = text;
  return element;
}

/** 状态圆点（配合状态文字使用）。 */
export function statusDot(colorClass: string): HTMLElement {
  const element = document.createElement("span");
  element.className = `inline-block size-1.5 shrink-0 rounded-full ${colorClass}`;
  return element;
}

type ButtonVariant = "primary" | "ghost" | "danger";

/** 统一按钮：primary 实心强调、ghost 描边、danger 危险描边。 */
export function button(label: string, variant: ButtonVariant = "ghost"): HTMLButtonElement {
  const element = document.createElement("button");
  element.type = "button";
  element.textContent = label;
  element.className = [
    "rounded-lg px-3 py-1.5 text-xs font-medium transition-colors",
    "disabled:cursor-not-allowed disabled:opacity-40",
    variant === "primary"
      ? "bg-emerald-500 text-zinc-950 shadow-sm shadow-emerald-500/20 hover:bg-emerald-400"
      : variant === "danger"
        ? "border border-zinc-700 text-zinc-400 hover:border-red-500/70 hover:text-red-400"
        : "border border-zinc-700 text-zinc-200 hover:border-emerald-500/70 hover:bg-emerald-500/10 hover:text-emerald-300",
  ].join(" ");
  return element;
}

const INPUT_BASE =
  "rounded-lg border border-zinc-700 bg-zinc-950 text-zinc-100 transition-colors placeholder:text-zinc-600 focus:border-emerald-500 focus:ring-2 focus:ring-emerald-500/20 focus:outline-none";

export function textInput(placeholder: string, extraClass = ""): HTMLInputElement {
  const element = document.createElement("input");
  element.type = "text";
  element.placeholder = placeholder;
  element.className = `${INPUT_BASE} px-3 py-1.5 text-sm ${extraClass}`.trim();
  return element;
}

/** 数字输入框：type=number 的 textInput 变体；min/step 等约束由调用方按需补充。 */
export function numberInput(placeholder: string, extraClass = ""): HTMLInputElement {
  const element = textInput(placeholder, extraClass);
  element.type = "number";
  return element;
}

/** 面板内紧凑下拉框；选项由调用方按需填充（含动态刷新场景）。 */
export function dropdown(extraClass = ""): HTMLSelectElement {
  const element = document.createElement("select");
  element.className = `${INPUT_BASE} px-2 py-1.5 text-xs ${extraClass}`.trim();
  return element;
}

/** 细进度条：0..100 百分比。 */
export function progressBar(percent: number): HTMLElement {
  const track = document.createElement("div");
  track.className = "h-1.5 overflow-hidden rounded-full bg-zinc-800";
  const fill = document.createElement("div");
  fill.className = "h-full rounded-full bg-emerald-500 transition-all";
  fill.style.width = `${Math.min(Math.max(percent, 0), 100)}%`;
  track.append(fill);
  return track;
}
