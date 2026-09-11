import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useCommandRegistry } from "../../../src-web/components/menu-bar/useCommandRegistry";
import { useAppStore } from "../../../src-web/stores/app";

vi.mock("../../../src-web/menu-actions", () => ({
  runMenuAction: vi.fn(),
}));

import { runMenuAction } from "../../../src-web/menu-actions";

const GROUP_LABELS = ["文件", "视图", "工具", "结果", "报告", "帮助"];

describe("useCommandRegistry", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.mocked(runMenuAction).mockClear();
  });

  it("六个一级菜单分组，与设计稿菜单结构一致", () => {
    const { menuGroups } = useCommandRegistry();
    expect(menuGroups.map((group) => group.label)).toEqual(GROUP_LABELS);
  });

  it("全量命令 = 七个阶段切换 + 菜单命令，均带分组名", () => {
    const { menuGroups, allCommands } = useCommandRegistry();
    const stageCommands = allCommands.filter((command) => command.group === "分析阶段");
    expect(stageCommands.map((command) => command.label)).toEqual([
      "主页",
      "几何",
      "网格",
      "工艺",
      "求解",
      "结果",
      "报告",
    ]);
    const menuCount = menuGroups.reduce((sum, group) => sum + group.commands.length, 0);
    expect(allCommands.length).toBe(7 + menuCount);
    expect(allCommands[0]!.group).toBe("分析阶段");
  });

  it("菜单命令触发对应的原生菜单动作 id（含关于）", () => {
    const { allCommands } = useCommandRegistry();
    for (const command of allCommands.filter((item) => item.group !== "分析阶段")) {
      command.run();
      expect(runMenuAction).toHaveBeenCalledWith(command.id);
    }
    expect(runMenuAction).toHaveBeenCalledWith("app.about");
  });

  it("阶段切换命令写入 store 且与阶段一一对应", () => {
    const { allCommands } = useCommandRegistry();
    const app = useAppStore();
    for (const command of allCommands.filter((item) => item.group === "分析阶段")) {
      command.run();
      expect(app.stage).toBe(command.id.replace("stage.", ""));
    }
  });
});
