/**
 * 运行时依赖面板：许可分级、就绪状态、应用内下载与官方页引导。
 * 徽标文案与配色正交：许可定颜色（合规口径），安装策略定文案（Gmsh = GPL + 官方直链）。
 */
import { computed } from "vue";
import { useAppStore } from "../../stores/app";
import { useVmStore } from "../../stores/vm";
import { useDependenciesStore } from "../../stores/dependencies";
import { isPendingDeploy } from "../../utils/deploy";
import type { DependencyStatus } from "../../types";
import { fixed } from "../../utils/format";

/** 单行依赖的展示模型：模板分支与文案集中此处算清，避免模板里重复取值。 */
interface DependencyRow {
  dep: DependencyStatus;
  already: boolean;
  downloading: boolean;
  percent: number;
  percentWidth: string;
  downloadUrl: string;
  downloadLabel: string;
  downloadVariant: "primary" | "ghost";
  downloadTitle: string;
  badgeText: string;
  badgeClass: string;
  readyText: string;
  readyClass: string;
  hasUpdate: boolean;
  checked: boolean;
  updateLineText: string;
  freshText: string;
  freshClass: string;
  failedText: string | null;
  downloadedText: string | null;
  extractDir: string | null;
  savedText: string | null;
}

export function useDependenciesPanel() {
  /** 徽标：许可定颜色（合规口径），安装策略定文案——两者正交（Gmsh = GPL + 官方直链）。 */
  const LICENSE_CLASS: Record<string, string> = {
    mit: "border-emerald-500/60 bg-emerald-500/10 text-emerald-300",
    gpl: "border-amber-500/60 bg-amber-500/10 text-amber-300",
  };

  const STRATEGY_LABEL: Record<string, string> = {
    direct_download: "官方直链 · 可直接下载",
    guided_install: "引导安装",
  };

  const app = useAppStore();
  const vm = useVmStore();
  const deps = useDependenciesStore();

  /** 取组件在当前平台的下载地址（OS 未就绪时回落 linux 源）。 */
  function updateDownloadUrl(dep: DependencyStatus): string {
    const os = app.info?.os ?? "linux";
    return dep.download === null
      ? ""
      : os === "macos"
        ? dep.download.macos
        : os === "windows"
          ? dep.download.windows
          : dep.download.linux;
  }

  const rows = computed<DependencyRow[]>(() =>
    deps.dependencies.map((dep) => {
      const stage = deps.componentStages[dep.id];
      const downloading = stage?.stage === "downloading";
      const percent = stage?.stage === "downloading" ? stage.percent : 0;
      const already = dep.id in deps.savedDownloads || dep.id in deps.downloadedFiles;
      const check = deps.updateChecks[dep.id];
      const downloaded = deps.downloadedFiles[dep.id];
      const saved = deps.savedDownloads[dep.id];
      const usable = dep.ready || dep.managedReady;
      const sizeMb = downloaded !== undefined ? fixed(downloaded.sizeBytes / 1024 / 1024, 1) : "";
      return {
        dep,
        already,
        downloading,
        percent,
        percentWidth: `${Math.min(Math.max(percent, 0), 100)}%`,
        downloadUrl: updateDownloadUrl(dep),
        downloadLabel: already ? "重新下载" : "下载",
        downloadVariant: already ? "ghost" : "primary",
        downloadTitle: `下载（${dep.license}）`,
        badgeText: STRATEGY_LABEL[dep.strategy] ?? dep.license,
        badgeClass: LICENSE_CLASS[dep.licenseKind] ?? "border-zinc-700 text-zinc-400",
        readyText: usable ? (dep.managedReady ? "就绪（应用内副本）" : "就绪") : "未就绪",
        readyClass: usable ? "text-emerald-400" : "text-red-400",
        hasUpdate: check?.updateAvailable === true,
        checked: check !== undefined,
        updateLineText:
          check?.updateAvailable === true
            ? `发现新版本 ${check.latestTag}（当前 ${check.installedTag ?? "未知"}）`
            : "",
        freshText:
          check !== undefined && check.updateAvailable !== true
            ? check.installedTag === null
              ? "已安装（版本标识未知，重新下载可获得版本标记）。"
              : `已是最新版本（${check.installedTag}）。`
            : "",
        freshClass:
          check !== undefined && check.updateAvailable !== true
            ? `text-xs ${check.installedTag === null ? "text-zinc-400" : "text-emerald-400"}`
            : "",
        failedText: stage?.stage === "failed" ? stage.error : null,
        downloadedText:
          downloaded !== undefined
            ? `已下载 ${downloaded.fileName}（${sizeMb} MB${downloaded.extractDir !== null ? "，已解压" : ""}）`
            : null,
        extractDir: downloaded?.extractDir ?? null,
        savedText:
          saved !== undefined
            ? saved.extractDir !== null
              ? `已解压：${saved.extractDir}`
              : `已保存：${saved.path}`
            : null,
      };
    }),
  );

  // 下载目录路径经依赖 store 取回（错误进全局管道），探测前展示占位文案。
  const downloadsDir = computed(() => deps.downloadsDir ?? "探测中…");
  void deps.refreshDownloadsDir();

  void deps.refreshDependencies();
  // VM 内 / 本机已部署版本（「更新未部署」提醒的比对源）；Linux 原生通道另探测环境
  void vm.refreshDeployedReleaseTag();
  if (vm.vmStatus?.provider === "native") {
    void vm.refreshNativeEnv();
  }

  /** 部署按钮文案：Linux 是「本机就位」而不是「部署到虚拟机」。 */
  const deployLabel = computed(() =>
    vm.vmStatus?.provider === "native" ? "部署到本机" : "部署到虚拟机",
  );
  /** 原生（Linux）环境提示行（空 = 无问题 / 非原生平台）。 */
  const nativeHints = computed(() =>
    vm.vmStatus?.provider === "native" ? (vm.nativeEnv?.hints ?? []) : [],
  );

  /** 待部署提示文案（版本从 → 到；VM 侧未知显示「未部署」）。 */
  const pendingDeployText = computed(() => {
    const downloaded = deps.downloadedFiles["moldingfoam"]?.releaseTag ?? null;
    if (downloaded === null || !isPendingDeploy(downloaded, vm.deployedReleaseTag)) {
      return null;
    }
    const where = vm.vmStatus?.provider === "native" ? "本机" : "VM 内";
    const action = vm.vmStatus?.provider === "native" ? "部署到本机" : "部署到虚拟机";
    return `求解环境有更新未部署：${downloaded} → ${where} ${vm.deployedReleaseTag ?? "未部署"}，请${action}后提交作业`;
  });

  /** 下载完成后（Linux 上解压即就位）刷新原生环境提示行。 */
  async function downloadComponent(componentId: string, url: string): Promise<void> {
    await deps.downloadComponent(componentId, url);
    if (vm.vmStatus?.provider === "native") {
      await vm.refreshNativeEnv();
    }
  }

  return {
    app,
    vm,
    deps,
    rows,
    downloadComponent,
    downloadsDir,
    pendingDeployText,
    deployLabel,
    nativeHints,
    openDownloadsDir: deps.openDownloadsDir,
  };
}
