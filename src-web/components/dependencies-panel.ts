import { appStore, openDependencyPageAction, refreshDependencies } from "../state";
import { button, card, hint } from "./ui";

const LICENSE_BADGE: Record<string, { label: string; className: string }> = {
  mit: {
    label: "MIT · 可直接下载",
    className: "border-emerald-500/60 bg-emerald-500/10 text-emerald-300",
  },
  gpl: { label: "GPL · 引导安装", className: "border-amber-500/60 bg-amber-500/10 text-amber-300" },
};

/** 运行时依赖面板：许可分级、就绪状态与官方页引导（MIT 可直接下载，GPL 引导安装）。 */
export function createDependenciesPanel(): HTMLElement {
  const { root, body } = card("运行时依赖");

  const refreshButton = button("重新探测");
  const listBox = document.createElement("div");
  listBox.className = "space-y-2";
  body.append(refreshButton, listBox);

  refreshButton.addEventListener("click", () => void refreshDependencies());

  function render(): void {
    const { dependencies, busy } = appStore.get();
    const working = busy !== null;
    refreshButton.disabled = working;

    listBox.replaceChildren();
    if (dependencies.length === 0) {
      listBox.append(hint("尚未探测。点击「重新探测」检查 OpenFOAM 环境与求解器。"));
      return;
    }
    for (const dep of dependencies) {
      const row = document.createElement("div");
      row.className =
        "flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg border border-zinc-800 px-3 py-2 text-xs";

      const name = document.createElement("span");
      name.className = "font-semibold text-zinc-200";
      name.textContent = dep.name;

      const badge = document.createElement("span");
      const badgeStyle = LICENSE_BADGE[dep.licenseKind] ?? {
        label: dep.license,
        className: "border-zinc-700 text-zinc-400",
      };
      badge.className = `rounded-full border px-2 py-0.5 ${badgeStyle.className}`;
      badge.textContent = badgeStyle.label;

      const ready = document.createElement("span");
      ready.className = dep.ready ? "text-emerald-400" : "text-red-400";
      ready.textContent = dep.ready ? "就绪 ✓" : "未就绪";

      const requiredTag = document.createElement("span");
      requiredTag.className = "text-zinc-600";
      requiredTag.textContent = dep.required ? "必需" : "可选";

      const openButton = button("官方页", "ghost");
      openButton.addEventListener("click", () => {
        void openDependencyPageAction(dep.pageUrl);
      });

      row.append(name, badge, ready, requiredTag, openButton);
      listBox.append(row);

      if (!dep.ready && dep.required) {
        const hintLine = hint(dep.hint);
        hintLine.className = "text-xs text-amber-400 pl-3";
        listBox.append(hintLine);
      }
    }
  }

  body.append(refreshButton, listBox);
  refreshDependencies();
  render();
  appStore.subscribe(render);
  return root;
}
