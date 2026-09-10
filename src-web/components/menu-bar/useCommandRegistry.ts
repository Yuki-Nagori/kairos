/**
 * 命令注册表：菜单栏一级菜单与命令面板（⌘K）共用的单一命令源。
 * 动作经 runMenuAction 路由（与原生菜单同一动作集），保证各入口行为一致；
 * 阶段切换在执行时现取 app store（命令闭包不绑定具体 pinia 实例）。
 * 无本地状态，多次调用彼此独立。
 */
import { useAppStore } from "../../stores/app";
import { runMenuAction } from "../../menu-actions";
import { STAGES } from "../stage-tabs/useStageTabs";

/** 单条命令：label 为展示名，shortcut 仅作提示，run 直接触发动作。 */
export interface CommandDef {
  id: string;
  label: string;
  shortcut?: string;
  run: () => void;
}

/** 一级菜单分组：label 为菜单名，commands 为其下的命令集合。 */
interface MenuGroup {
  label: string;
  commands: CommandDef[];
}

/** 命令面板条目：命令 + 所属分组名（用于搜索匹配与分组展示）。 */
export interface PaletteItem extends CommandDef {
  group: string;
}

/** 关于对话框的唤起事件：注册表只发事件，UI 显隐由 useMenuBar 消费。 */
export const ABOUT_EVENT = "kairos:show-about";

export function useCommandRegistry() {
  const menuGroups: MenuGroup[] = [
    {
      label: "文件",
      commands: [
        { id: "file.new", label: "新建项目", shortcut: "⌘N", run: () => runMenuAction("file.new") },
        {
          id: "file.open",
          label: "打开项目…",
          shortcut: "⌘O",
          run: () => runMenuAction("file.open"),
        },
        { id: "file.save", label: "保存", shortcut: "⌘S", run: () => runMenuAction("file.save") },
        {
          id: "file.saveAs",
          label: "另存为…",
          shortcut: "⇧⌘S",
          run: () => runMenuAction("file.saveAs"),
        },
      ],
    },
    {
      label: "视图",
      commands: [
        { id: "view.theme", label: "切换主题", run: () => runMenuAction("view.theme") },
        { id: "tools.vmPanel", label: "Shell 环境面板", run: () => runMenuAction("tools.vmPanel") },
      ],
    },
    {
      label: "分析",
      commands: [
        {
          id: "analysis.checkNetwork",
          label: "校验模具网络",
          run: () => runMenuAction("analysis.checkNetwork"),
        },
      ],
    },
    {
      label: "工具",
      commands: [
        {
          id: "tools.refreshDeps",
          label: "探测运行时依赖",
          run: () => runMenuAction("tools.refreshDeps"),
        },
        { id: "tools.vmStart", label: "启动虚拟机", run: () => runMenuAction("tools.vmStart") },
        {
          id: "tools.vmShell",
          label: "进入虚拟机 Shell",
          run: () => runMenuAction("tools.vmShell"),
        },
        { id: "tools.vmStop", label: "关闭虚拟机", run: () => runMenuAction("tools.vmStop") },
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
          id: "stage.report",
          label: "打开报告工作台",
          run: () => (useAppStore().stage = "report"),
        },
      ],
    },
    {
      label: "帮助",
      commands: [
        {
          id: "app.about",
          label: "关于 Kairos",
          run: () => window.dispatchEvent(new CustomEvent(ABOUT_EVENT)),
        },
      ],
    },
  ];

  /** 阶段切换命令：命令面板里按工作流直达任一分析阶段（store 在执行时现取）。 */
  const stageCommands: PaletteItem[] = STAGES.map(([stage, label]) => ({
    id: `stage.${stage}`,
    label,
    group: "分析阶段",
    run: () => (useAppStore().stage = stage),
  }));

  /** 全量命令（阶段切换在前，菜单命令随后）：命令面板的搜索域。 */
  const allCommands: PaletteItem[] = [
    ...stageCommands,
    ...menuGroups.flatMap((group) =>
      group.commands.map((command) => ({ ...command, group: group.label })),
    ),
  ];

  return { menuGroups, allCommands };
}
