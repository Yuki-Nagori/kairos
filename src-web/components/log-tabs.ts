/**
 * 中列底部日志标签组（v2 设计稿）：分析日志 / 网格日志 / VM 终端。
 * 只读镜像——交互式输入仍在 VM 抽屉；数据全部来自 store 分片。
 */
import { appStore } from "../state";

const TABS = ["分析日志", "网格日志", "VM 终端"] as const;
type Tab = (typeof TABS)[number];

export function createLogTabs(): HTMLElement {
  const root = document.createElement("div");
  root.className =
    "mt-2 flex h-44 flex-col overflow-hidden rounded-lg border border-zinc-800 bg-black";
  const tabbar = document.createElement("div");
  tabbar.className =
    "flex items-center gap-2 border-b border-zinc-800 bg-zinc-900 px-3 py-1.5 text-[10px] text-zinc-400";
  const content = document.createElement("pre");
  content.className =
    "flex-1 overflow-y-auto whitespace-pre-wrap break-all px-3 py-2 text-[11px] leading-4 text-emerald-300/90";
  root.append(tabbar, content);

  let active: Tab = TABS[0];
  const buttons = TABS.map((label) => {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = label;
    btn.className = "rounded px-2 py-0.5 text-zinc-500 hover:text-zinc-200";
    btn.addEventListener("click", () => {
      active = label;
      render();
    });
    tabbar.append(btn);
    return btn;
  });

  function lines(): string[] {
    const { jobLogs, jobs, meshReports, vmShellLogs } = appStore.get();
    if (active === "分析日志") {
      const job = jobs.at(-1);
      return (job ? (jobLogs[job.id] ?? []) : []).slice(-40);
    }
    if (active === "网格日志") {
      const report = Object.values(meshReports).at(-1);
      return report
        ? [
            `引擎 ${report.engine} · ${report.nodeCount} 节点 / ${report.elementCount} 四面体`,
            `质量（长径比）min ${report.quality.minEdgeRatio.toFixed(2)} / avg ${report.quality.avgEdgeRatio.toFixed(2)} / max ${report.quality.maxEdgeRatio.toFixed(2)}`,
          ]
        : [];
    }
    return vmShellLogs.slice(-40);
  }

  function render(): void {
    for (const btn of buttons) {
      const on = btn.textContent === active;
      btn.className = `rounded px-2 py-0.5 ${
        on ? "bg-zinc-800 text-emerald-400" : "text-zinc-500 hover:text-zinc-200"
      }`;
    }
    const list = lines();
    content.textContent = list.length > 0 ? list.join("\n") : "（暂无日志）";
  }

  render();
  appStore.subscribe(render);
  return root;
}
