import {
  appStore,
  checkUpdateAction,
  deployVmBundleAction,
  downloadComponent,
  openDependencyPageAction,
  refreshDependencies,
} from "../../state";
import { openDownloadsDir } from "../../services/downloads";
import { button, card, hint, progressBar } from "../ui";

/** 徽标：许可定颜色（合规口径），安装策略定文案——两者正交（Gmsh = GPL + 官方直链）。 */
const LICENSE_CLASS: Record<string, string> = {
  mit: "border-emerald-500/60 bg-emerald-500/10 text-emerald-300",
  gpl: "border-amber-500/60 bg-amber-500/10 text-amber-300",
};

const STRATEGY_LABEL: Record<string, string> = {
  direct_download: "官方直链 · 可直接下载",
  guided_install: "引导安装",
};

/** 取组件在当前平台的下载地址（OS 未就绪时回落 linux 源）。 */
function updateDownloadUrl(dep: {
  download: { macos: string; windows: string; linux: string } | null;
}): string {
  const os = appStore.get().info?.os ?? "linux";
  return dep.download === null
    ? ""
    : os === "macos"
      ? dep.download.macos
      : os === "windows"
        ? dep.download.windows
        : dep.download.linux;
}

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

  void import("../../services/downloads").then(async (m) => {
    dirLine.textContent = `下载目录：${await m.getDownloadsDir()}`;
  });

  function render(): void {
    const { dependencies, busy, savedDownloads, downloadedFiles, componentStages, updateChecks } =
      appStore.get();
    const working = busy !== null;
    refreshButton.disabled = working;
    openDirButton.disabled = working;

    listBox.replaceChildren();
    if (dependencies.length === 0) {
      listBox.append(hint("尚未探测。点击「重新探测」检查 OpenFOAM 环境。"));
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
      badge.className = `inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[10px] ${
        LICENSE_CLASS[dep.licenseKind] ?? "border-zinc-700 text-zinc-400"
      }`;
      badge.textContent = STRATEGY_LABEL[dep.strategy] ?? dep.license;

      const ready = document.createElement("span");
      const usable = dep.ready || dep.managedReady;
      ready.className = usable ? "text-emerald-400" : "text-red-400";
      ready.textContent = usable ? (dep.managedReady ? "就绪（应用内副本）" : "就绪") : "未就绪";

      const requiredTag = document.createElement("span");
      requiredTag.className = "text-zinc-600";
      requiredTag.textContent = dep.required ? "必需" : "可选";

      const openButton = button("官方页", "ghost");
      openButton.addEventListener("click", () => {
        void openDependencyPageAction(dep.pageUrl);
      });

      // 有应用内下载地址的组件：提供「下载」按钮（用户点击触发，官方源 + 许可展示）。
      // 按钮可用性从统一阶段状态推导：下载进行中禁用。
      const stage = componentStages[dep.id];
      const downloading = stage?.stage === "downloading";
      const already = dep.id in savedDownloads || dep.id in downloadedFiles;
      let downloadButton: HTMLButtonElement | null = null;
      if (dep.download !== null) {
        const os = appStore.get().info?.os ?? "linux";
        const downloadUrl =
          os === "macos"
            ? dep.download.macos
            : os === "windows"
              ? dep.download.windows
              : dep.download.linux;
        downloadButton = button(already ? "重新下载" : "下载", already ? "ghost" : "primary");
        downloadButton.disabled = downloading;
        downloadButton.title = `下载（${dep.license}）`;
        downloadButton.addEventListener("click", () => {
          void downloadComponent(dep.id, downloadUrl);
        });
      }

      // 求解环境 bundle：已下载后提供「部署到虚拟机」（multipass 平台）。
      if (dep.id === "moldingfoam" && already) {
        const deployButton = button("部署到虚拟机", "ghost");
        deployButton.addEventListener("click", () => {
          void deployVmBundleAction();
        });
        row.append(deployButton);
      }

      // 已安装的 release 流组件：提供「检查更新」。
      if (dep.updatable && already) {
        const checkButton = button("检查更新", "ghost");
        checkButton.addEventListener("click", () => {
          void checkUpdateAction(dep.id);
        });
        row.append(checkButton);
      }

      row.append(name, badge, ready, requiredTag, openButton);
      if (downloadButton !== null) {
        row.append(downloadButton);
      }
      listBox.append(row);

      // 更新提示：发现新版（琥珀 + 更新按钮）→ 已是最新（绿）→ 版本未知（灰）。
      const check = updateChecks[dep.id];
      if (check?.updateAvailable === true) {
        const updateLine = hint(
          `发现新版本 ${check.latestTag}（当前 ${check.installedTag ?? "未知"}）`,
        );
        updateLine.className = "text-xs text-amber-400";
        listBox.append(updateLine);
        const updateButton = button("更新到新版");
        updateButton.addEventListener("click", () => {
          void downloadComponent(dep.id, updateDownloadUrl(dep));
        });
        listBox.append(updateButton);
      } else if (check !== undefined) {
        const freshLine = hint(
          check.installedTag === null
            ? "已安装（版本标识未知，重新下载可获得版本标记）。"
            : `已是最新版本（${check.installedTag}）。`,
        );
        freshLine.className = `text-xs ${
          check.installedTag === null ? "text-zinc-400" : "text-emerald-400"
        }`;
        listBox.append(freshLine);
      }

      // 阶段状态行：失败（红，附原因）→ 下载进度。
      if (stage?.stage === "failed") {
        const failedLine = hint(stage.error);
        failedLine.className = "text-xs text-red-400";
        listBox.append(failedLine);
      }
      if (downloading) {
        const percent = stage?.stage === "downloading" ? stage.percent : 0;
        const progressWrap = document.createElement("div");
        progressWrap.className = "flex items-center gap-2 pl-3";
        progressWrap.append(progressBar(percent));
        const progressLine = hint(`${percent}%`);
        progressLine.className = "w-9 text-[10px] tabular-nums text-zinc-400";
        progressWrap.append(progressLine);
        listBox.append(progressWrap);
      }

      const downloaded = downloadedFiles[dep.id];
      if (downloaded !== undefined) {
        const sizeMb = (downloaded.sizeBytes / 1024 / 1024).toFixed(1);
        const tail = downloaded.extractDir !== null ? `，已解压` : "";
        const doneLine = hint(`已下载 ${downloaded.fileName}（${sizeMb} MB${tail}）`);
        doneLine.className = "text-xs text-emerald-400";
        listBox.append(doneLine);
        if (downloaded.extractDir !== null) {
          const dirLine = hint(`解压目录：${downloaded.extractDir}`);
          listBox.append(dirLine);
        }
      }

      const saved = savedDownloads[dep.id];
      if (saved !== undefined) {
        const savedLine = hint(
          saved.extractDir !== null ? `已解压：${saved.extractDir}` : `已保存：${saved.path}`,
        );
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
