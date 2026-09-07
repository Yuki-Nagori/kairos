import {
  appStore,
  downloadComponent,
  openDependencyPageAction,
  refreshDependencies,
} from "../state";
import { openDownloadsDir } from "../services/downloads";
import { button, card, hint } from "./ui";

const LICENSE_BADGE: Record<string, { label: string; className: string }> = {
  mit: {
    label: "MIT · 可直接下载",
    className: "border-emerald-500/60 bg-emerald-500/10 text-emerald-300",
  },
  gpl: { label: "GPL · 引导安装", className: "border-amber-500/60 bg-amber-500/10 text-amber-300" },
};

/** 运行时依赖面板：许可分级、就绪状态、应用内下载与官方页引导。 */
export function createDependenciesPanel(): HTMLElement {
  const { root, body } = card("运行时依赖");

  const refreshButton = button("重新探测");
  const openDirButton = button("打开下载目录");
  const listBox = document.createElement("div");
  listBox.className = "space-y-2";
  const dirLine = hint("下载目录：探测中…");
  body.append(refreshButton, openDirButton, dirLine, listBox);

  refreshButton.addEventListener("click", () => void refreshDependencies());
  openDirButton.addEventListener("click", () => void openDownloadsDir());

  void import("../services/downloads").then(async (m) => {
    dirLine.textContent = `下载目录：${await m.getDownloadsDir()}`;
  });

  function render(): void {
    const { dependencies, busy, downloadProgress, savedDownloads } = appStore.get();
    const working = busy !== null;
    refreshButton.disabled = working;
    openDirButton.disabled = working;

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

      // 有应用内下载地址的组件：提供「下载」按钮（用户点击触发，官方源 + 许可展示）。
      let downloadButton: HTMLButtonElement | null = null;
      if (dep.download !== null) {
        const os = appStore.get().info?.os ?? "linux";
        const downloadUrl =
          os === "macos"
            ? dep.download.macos
            : os === "windows"
              ? dep.download.windows
              : dep.download.linux;
        downloadButton = button("下载", "primary");
        downloadButton.title = `下载（${dep.license}）`;
        downloadButton.addEventListener("click", () => {
          void downloadComponent(dep.id, downloadUrl);
        });
      }

      row.append(name, badge, ready, requiredTag, openButton);
      if (downloadButton !== null) {
        row.append(downloadButton);
      }
      listBox.append(row);

      const progress = downloadProgress[dep.id];
      if (progress !== undefined) {
        const progressLine = hint(`下载中：${progress}%`);
        listBox.append(progressLine);
      }

      const saved = savedDownloads[dep.id];
      if (saved !== undefined) {
        const savedLine = hint(`已保存：${saved.path}`);
        listBox.append(savedLine);
      }

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
