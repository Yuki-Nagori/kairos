import { cycleTheme, getActiveTheme } from "../theme";
import { select } from "../lib/store";
import { appStore } from "../state";

const STATUS_BASE_CLASS = "min-h-5 text-sm";

/** 应用标题栏：品牌信息 + 主题切换 + 版本/IPC 状态 + 全局的忙碌与错误展示。 */
export function createAppHeader(): HTMLElement {
  const root = document.createElement("header");
  root.className = "flex items-center gap-4 px-5 py-3 border-b border-zinc-800";

  const title = document.createElement("h1");
  title.className = "text-lg font-semibold tracking-tight";
  title.textContent = "Kairos";

  const subtitle = document.createElement("span");
  subtitle.className = "text-xs text-zinc-500";
  subtitle.textContent = "CAE 仿真 · 框架搭建中";

  const spacer = document.createElement("span");
  spacer.className = "flex-1";

  const status = document.createElement("span");
  status.className = STATUS_BASE_CLASS;

  const themeButton = document.createElement("button");
  themeButton.type = "button";
  themeButton.className =
    "rounded-lg border border-zinc-700 px-2 py-1 text-xs text-zinc-300 hover:border-emerald-500 hover:text-emerald-300 transition-colors";

  function syncThemeIcon(): void {
    const theme = getActiveTheme();
    themeButton.textContent = theme === "light" ? "\u263d" : "\u2600";
    themeButton.title =
      theme === "light"
        ? "\u5207\u6362\u5230\u6d45\u8272\u4e3b\u9898"
        : "\u5207\u6362\u5230\u6df1\u8272\u4e3b\u9898";
  }

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

  themeButton.addEventListener("click", () => {
    cycleTheme();
    syncThemeIcon();
  });

  syncThemeIcon();
  root.append(title, subtitle, themeButton, status);
  return root;
}
