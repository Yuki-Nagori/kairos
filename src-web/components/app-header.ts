import { cycleTheme, getActiveTheme, THEME_CHANGED_EVENT } from "../theme";
import { select } from "../lib/store";
import { appStore } from "../state";

/** 应用标题栏：品牌信息 + 主题切换（全局状态展示在底部状态栏）。 */
export function createAppHeader(): HTMLElement {
  const root = document.createElement("header");
  root.className =
    "flex shrink-0 items-center gap-3 border-b border-zinc-800 bg-zinc-900 px-4 py-2";

  const logo = document.createElement("img");
  logo.src = "/icon.svg";
  logo.alt = "Kairos";
  logo.className = "h-6 w-6 shrink-0";

  const title = document.createElement("h1");
  title.className = "text-sm font-semibold tracking-wide";
  title.textContent = "Kairos";

  const subtitle = document.createElement("span");
  subtitle.className = "text-xs text-zinc-500";
  subtitle.textContent = "CAE 仿真";

  const spacer = document.createElement("span");
  spacer.className = "flex-1";

  const themeButton = document.createElement("button");
  themeButton.type = "button";
  themeButton.className =
    "rounded-lg border border-zinc-700 px-2 py-1 text-xs text-zinc-300 transition-colors hover:border-emerald-500/70 hover:bg-emerald-500/10 hover:text-emerald-300";

  function syncThemeIcon(): void {
    const theme = getActiveTheme();
    themeButton.textContent = theme === "light" ? "\u263d" : "\u2600";
    themeButton.title =
      theme === "light"
        ? "\u5207\u6362\u5230\u6d45\u8272\u4e3b\u9898"
        : "\u5207\u6362\u5230\u6df1\u8272\u4e3b\u9898";
  }

  themeButton.addEventListener("click", () => {
    // cycleTheme 广播 THEME_CHANGED_EVENT，图标经下方监听同步。
    cycleTheme();
  });

  window.addEventListener(THEME_CHANGED_EVENT, syncThemeIcon);

  syncThemeIcon();
  root.append(logo, title, subtitle, spacer, themeButton);
  return root;
}

const STATUS_BASE_CLASS = "text-[11px]";

/** 底部状态栏：版本/IPC、忙碌、错误三态（优先级：错误 > 忙碌 > 版本）。 */
export function createStatusBar(): HTMLElement {
  const root = document.createElement("footer");
  root.className =
    "flex shrink-0 items-center justify-between gap-4 border-t border-zinc-800 bg-zinc-900 px-4 py-1.5";

  const status = document.createElement("span");
  status.className = STATUS_BASE_CLASS;

  const spacer = document.createElement("span");
  spacer.className = "flex-1";

  function renderStatus(): void {
    const state = appStore.get();
    if (state.error) {
      status.textContent = state.error.message;
      status.className = state.error.info
        ? `${STATUS_BASE_CLASS} text-zinc-400`
        : `${STATUS_BASE_CLASS} text-red-400`;
    } else if (state.busy) {
      status.textContent = state.busy;
      status.className = `${STATUS_BASE_CLASS} text-amber-300`;
    } else if (state.info) {
      status.textContent = `v${state.info.version} \u00b7 ${state.info.os} \u00b7 IPC \u6b63\u5e38`;
      status.className = `${STATUS_BASE_CLASS} text-zinc-500`;
    } else {
      status.textContent = "";
      status.className = STATUS_BASE_CLASS;
    }
  }

  select(appStore, (s) => s.error, renderStatus);
  select(appStore, (s) => s.busy, renderStatus);
  select(appStore, (s) => s.info, renderStatus);

  root.append(status, spacer);
  return root;
}
