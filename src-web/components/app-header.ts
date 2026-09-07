import { select } from "../lib/store";
import { appStore } from "../state";

const STATUS_BASE_CLASS = "min-h-5 text-sm";

/** 应用标题栏：品牌信息 + 版本/IPC 状态 + 全局的忙碌与错误展示。 */
export function createAppHeader(): HTMLElement {
  const root = document.createElement("header");
  root.className = "space-y-1";

  const title = document.createElement("h1");
  title.className = "text-2xl font-semibold tracking-tight";
  title.textContent = "Kairos";
  const subtitle = document.createElement("p");
  subtitle.className = "text-sm text-zinc-400";
  subtitle.textContent = "CAE 仿真软件 · 框架搭建中";

  const status = document.createElement("p");
  status.className = STATUS_BASE_CLASS;

  // 错误 → 忙碌 → 版本信息，三类状态互斥展示；select 只在对应切片变化时触发。
  function renderStatus(): void {
    const state = appStore.get();
    if (state.error) {
      status.textContent = state.error.message;
      // info 级是环境提示（浏览器预览），中性色；只有真实失败才用红色
      status.className = state.error.info
        ? `${STATUS_BASE_CLASS} text-zinc-400`
        : `${STATUS_BASE_CLASS} text-red-400`;
    } else if (state.busy) {
      status.textContent = state.busy;
      status.className = `${STATUS_BASE_CLASS} text-amber-300`;
    } else if (state.info) {
      status.textContent = `v${state.info.version} · ${state.info.os} · IPC 正常`;
      status.className = `${STATUS_BASE_CLASS} text-zinc-500`;
    } else {
      status.textContent = "";
      status.className = STATUS_BASE_CLASS;
    }
  }
  select(appStore, (state) => state.error, renderStatus);
  select(appStore, (state) => state.busy, renderStatus);
  select(appStore, (state) => state.info, renderStatus);

  root.append(title, subtitle, status);
  return root;
}
