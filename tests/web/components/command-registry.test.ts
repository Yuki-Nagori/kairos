import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import {
  ABOUT_EVENT,
  useCommandRegistry,
} from "../../../src-web/components/menu-bar/useCommandRegistry";
import { useAppStore } from "../../../src-web/stores/app";

vi.mock("../../../src-web/menu-actions", () => ({
  runMenuAction: vi.fn(),
}));

import { runMenuAction } from "../../../src-web/menu-actions";

const GROUP_LABELS = ["文件", "视图", "分析", "工具", "结果", "报告", "帮助"];

describe("useCommandRegistry", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.mocked(runMenuAction).mockClear();
  });

  it("七个一级菜单分组齐全", () => {
    const { menuGroups } = useCommandRegistry();
    expect(menuGroups.map((group) => group.label)).toEqual(GROUP_LABELS);
  });

  it("菜单命令触发对应动作 id", () => {
    const { menuGroups } = useCommandRegistry();
    for (const group of menuGroups) {
      for (const command of group.commands) {
        if (command.id.startsWith("stage.") || command.id === "app.about") {
          continue;
        }
        command.run();
        expect(runMenuAction).toHaveBeenCalledWith(command.id);
      }
    }
  });

  it("报告命令切换到报告阶段", () => {
    const { menuGroups } = useCommandRegistry();
    const report = menuGroups
      .find((group) => group.label === "报告")!
      .commands.find((command) => command.id === "stage.report")!;
    report.run();
    expect(useAppStore().stage).toBe("report");
  });

  it("关于命令广播 ABOUT_EVENT", () => {
    const { menuGroups } = useCommandRegistry();
    const about = menuGroups
      .find((group) => group.label === "帮助")!
      .commands.find((command) => command.id === "app.about")!;

    const listener = vi.fn();
    window.addEventListener(ABOUT_EVENT, listener);
    about.run();
    window.removeEventListener(ABOUT_EVENT, listener);
    expect(listener).toHaveBeenCalledOnce();
  });

  it("全量命令 = 阶段切换 + 菜单命令，且带分组名", () => {
    const { menuGroups, allCommands } = useCommandRegistry();
    const menuCount = menuGroups.reduce((sum, group) => sum + group.commands.length, 0);
    expect(allCommands.length).toBe(7 + menuCount);
    expect(allCommands[0]!.group).toBe("分析阶段");
    expect(allCommands.filter((command) => command.id === "stage.home").length).toBe(1);
  });

  it("阶段切换命令覆盖七个阶段并写入 store", () => {
    const { allCommands } = useCommandRegistry();
    const app = useAppStore();
    for (const command of allCommands.filter((item) => item.group === "分析阶段")) {
      command.run();
      expect(app.stage).toBe(command.id.replace("stage.", ""));
    }
  });
});
