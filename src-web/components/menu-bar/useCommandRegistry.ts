/**
 * 命令注册表：命令面板（⌘K）的单一命令源。
 * 菜单本体在系统菜单栏（Tauri 原生菜单），动作经 runMenuAction 路由
 * 与原生菜单共用同一动作集；阶段切换在执行时现取 app store
 * （命令闭包不绑定具体 pinia 实例）。无本地状态，多次调用彼此独立。
 */
import { useAppStore } from "../../stores/app";
import { runMenuAction } from "../../menu-actions";
import { STAGES } from "../stage-tabs/useStageTabs";

/** 命令面板条目：label 为展示名，group 为分组名，shortcut 仅作提示。 */
export interface PaletteItem {
  id: string;
  label: string;
  group: string;
  shortcut?: string;
  run: () => void;
}

export function useCommandRegistry() {
  /** 阶段切换命令：按工作流直达任一分析阶段。 */
  const stageCommands: PaletteItem[] = STAGES.map(([stage, label]) => ({
    id: `stage.${stage}`,
    label,
    group: "分析阶段",
    run: () => (useAppStore().stage = stage),
  }));

  /** 菜单命令：与原生菜单同一动作集（id 即 menu-action 事件里的动作 id）。 */
  const menuCommands: PaletteItem[] = [
    {
      id: "file.new",
      label: "新建项目",
      group: "文件",
      shortcut: "⌘N",
      run: () => runMenuAction("file.new"),
    },
    {
      id: "file.open",
      label: "打开项目…",
      group: "文件",
      shortcut: "⌘O",
      run: () => runMenuAction("file.open"),
    },
    {
      id: "file.save",
      label: "保存",
      group: "文件",
      shortcut: "⌘S",
      run: () => runMenuAction("file.save"),
    },
    {
      id: "file.saveAs",
      label: "另存为…",
      group: "文件",
      shortcut: "⇧⌘S",
      run: () => runMenuAction("file.saveAs"),
    },
    { id: "view.theme", label: "切换主题", group: "视图", run: () => runMenuAction("view.theme") },
    {
      id: "tools.vmPanel",
      label: "Shell 环境面板",
      group: "视图",
      run: () => runMenuAction("tools.vmPanel"),
    },
    {
      id: "analysis.checkNetwork",
      label: "校验模具网络",
      group: "工具",
      run: () => runMenuAction("analysis.checkNetwork"),
    },
    {
      id: "tools.refreshDeps",
      label: "探测运行时依赖",
      group: "工具",
      run: () => runMenuAction("tools.refreshDeps"),
    },
    {
      id: "tools.vmStart",
      label: "启动虚拟机",
      group: "工具",
      run: () => runMenuAction("tools.vmStart"),
    },
    {
      id: "tools.vmShell",
      label: "进入虚拟机 Shell",
      group: "工具",
      run: () => runMenuAction("tools.vmShell"),
    },
    {
      id: "tools.vmStop",
      label: "关闭虚拟机",
      group: "工具",
      run: () => runMenuAction("tools.vmStop"),
    },
    {
      id: "results.exportCsv",
      label: "导出当前场为 CSV",
      group: "结果",
      run: () => runMenuAction("results.exportCsv"),
    },
    {
      id: "report.open",
      label: "打开报告工作台",
      group: "报告",
      run: () => runMenuAction("report.open"),
    },
  ];

  /** 全量命令（阶段切换在前，菜单命令随后）：命令面板的搜索域。 */
  const allCommands: PaletteItem[] = [...stageCommands, ...menuCommands];

  return { allCommands };
}
