/**
 * 命令注册表：标题栏菜单（Windows/Linux 自绘）与命令面板（⌘K / Ctrl+K）
 * 共用的单一命令源。动作经 runMenuAction 路由（macOS 原生菜单同一动作集，
 * id 即 menu-action 事件里的动作 id）；阶段切换在执行时现取 app store
 * （命令闭包不绑定具体 pinia 实例）。无本地状态，多次调用彼此独立。
 */
import { useAppStore } from "../../stores/app";
import { runMenuAction } from "../../menu-actions";
import { shortcutLabel, SHORTCUTS } from "../../utils/shortcuts";
import { STAGES } from "../stage-tabs/useStageTabs";

/** 命令面板条目：label 为展示名，group 为分组名，shortcut 仅作提示。 */
export interface PaletteItem {
  id: string;
  label: string;
  group: string;
  shortcut?: string;
  run: () => void;
}

/** 菜单命令：分组名由所属一级菜单提供，命令本体不带 group。 */
export type MenuCommand = Omit<PaletteItem, "group">;

/** 一级菜单分组：label 为菜单名，commands 为其下的命令集合。 */
interface MenuGroup {
  label: string;
  commands: MenuCommand[];
}

export function useCommandRegistry() {
  /** 阶段切换命令：按工作流直达任一分析阶段。 */
  const stageCommands: PaletteItem[] = STAGES.map(([stage, label]) => ({
    id: `stage.${stage}`,
    label,
    group: "分析阶段",
    run: () => (useAppStore().stage = stage),
  }));

  const menuGroups: MenuGroup[] = [
    {
      label: "文件",
      commands: [
        {
          id: "file.new",
          label: "新建项目",
          shortcut: shortcutLabel(SHORTCUTS.fileNew),
          run: () => runMenuAction("file.new"),
        },
        {
          id: "file.open",
          label: "打开项目…",
          shortcut: shortcutLabel(SHORTCUTS.fileOpen),
          run: () => runMenuAction("file.open"),
        },
        {
          id: "file.save",
          label: "保存",
          shortcut: shortcutLabel(SHORTCUTS.fileSave),
          run: () => runMenuAction("file.save"),
        },
        {
          id: "file.saveAs",
          label: "另存为…",
          shortcut: shortcutLabel(SHORTCUTS.fileSaveAs),
          run: () => runMenuAction("file.saveAs"),
        },
      ],
    },
    {
      label: "视图",
      commands: [
        { id: "view.theme", label: "切换主题", run: () => runMenuAction("view.theme") },
        {
          id: "tools.vmPanel",
          label: "Shell 环境面板",
          run: () => runMenuAction("tools.vmPanel"),
        },
      ],
    },
    {
      label: "工具",
      commands: [
        {
          id: "analysis.checkNetwork",
          label: "校验模具网络",
          run: () => runMenuAction("analysis.checkNetwork"),
        },
        {
          id: "tools.refreshDeps",
          label: "探测运行时依赖",
          run: () => runMenuAction("tools.refreshDeps"),
        },
        {
          id: "tools.vmStart",
          label: "启动虚拟机",
          run: () => runMenuAction("tools.vmStart"),
        },
        {
          id: "tools.vmShell",
          label: "进入虚拟机 Shell",
          run: () => runMenuAction("tools.vmShell"),
        },
        {
          id: "tools.vmStop",
          label: "关闭虚拟机",
          run: () => runMenuAction("tools.vmStop"),
        },
      ],
    },
    {
      label: "结果",
      commands: [
        {
          id: "results.exportCsv",
          label: "导出当前场为 CSV",
          run: () => runMenuAction("results.exportCsv"),
        },
      ],
    },
    {
      label: "报告",
      commands: [
        {
          id: "report.open",
          label: "打开报告工作台",
          run: () => runMenuAction("report.open"),
        },
      ],
    },
    {
      label: "帮助",
      commands: [
        {
          id: "app.about",
          label: "关于 Kairos",
          run: () => runMenuAction("app.about"),
        },
      ],
    },
  ];

  /** 菜单命令平铺（带分组名）。 */
  const menuCommands: PaletteItem[] = menuGroups.flatMap((group) =>
    group.commands.map((command) => ({ ...command, group: group.label })),
  );

  /** 全量命令（阶段切换在前，菜单命令随后）：命令面板的搜索域。 */
  const allCommands: PaletteItem[] = [...stageCommands, ...menuCommands];

  return { menuGroups, allCommands };
}
