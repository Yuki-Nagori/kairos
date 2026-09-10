import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useCommandRegistry } from "../../../src-web/components/menu-bar/useCommandRegistry";
import { useAppStore } from "../../../src-web/stores/app";

vi.mock("../../../src-web/menu-actions", () => ({
  runMenuAction: vi.fn(),
}));

import { runMenuAction } from "../../../src-web/menu-actions";

describe("useCommandRegistry", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.mocked(runMenuAction).mockClear();
  });

  it("全量命令 = 七个阶段切换 + 菜单命令，均带分组名", () => {
    const { allCommands } = useCommandRegistry();
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
    // 菜单命令分组与原生菜单结构一致（设计稿：文件/视图/工具/结果/报告）
    const groups = [...new Set(allCommands.map((command) => command.group))];
    expect(groups).toEqual(["分析阶段", "文件", "视图", "工具", "结果", "报告"]);
  });

  it("菜单命令触发对应的原生菜单动作 id", () => {
    const { allCommands } = useCommandRegistry();
    for (const command of allCommands.filter((item) => item.group !== "分析阶段")) {
      command.run();
      expect(runMenuAction).toHaveBeenCalledWith(command.id);
    }
  });

  it("报告命令与原生菜单同走 report.open 动作", () => {
    const { allCommands } = useCommandRegistry();
    const report = allCommands.find((command) => command.label === "打开报告工作台")!;
    expect(report.group).toBe("报告");
    report.run();
    expect(runMenuAction).toHaveBeenCalledWith("report.open");
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
