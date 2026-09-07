/** 共享 DOM 构建辅助：设计 tokens 集中在 theme.ts（T16）。 */
import { PANEL_STATE_PREFIX, tokens } from "../theme";

export function card(
  title: string,
  options?: { collapsible?: boolean },
): { root: HTMLElement; body: HTMLElement } {
  const root = document.createElement("section");
  root.className = tokens.panel;
  const heading = document.createElement("h2");
  heading.className = tokens.panelHeader;
  heading.textContent = title;
  const body = document.createElement("div");
  body.className = tokens.panelBody;

  if (options?.collapsible === true) {
    const storageKey = `${PANEL_STATE_PREFIX}${title}`;
    const collapsed = localStorage.getItem(storageKey) === "1";
    body.classList.toggle("hidden", collapsed);
    heading.textContent = `${collapsed ? "▸" : "▾"} ${title}`;
    heading.addEventListener("click", () => {
      const nowCollapsed = !body.classList.contains("hidden");
      body.classList.toggle("hidden", nowCollapsed);
      heading.textContent = `${nowCollapsed ? "▸" : "▾"} ${title}`;
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
  element.className = tokens.input;
  return element;
}
