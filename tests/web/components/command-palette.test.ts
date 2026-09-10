import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import CommandPalette from "../../../src-web/components/command-palette/CommandPalette.vue";
import { useCommandPalette } from "../../../src-web/components/command-palette/useCommandPalette";
import { useAppStore } from "../../../src-web/stores/app";

/** 以全局 keydown 派发按键（面板监听在 window 上）。 */
function pressKey(key: string, modifiers: { metaKey?: boolean; ctrlKey?: boolean } = {}): void {
  window.dispatchEvent(new KeyboardEvent("keydown", { key, ...modifiers }));
}

describe("useCommandPalette", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    useCommandPalette().closePalette();
  });

  it("⌘K / Ctrl+K 切换开合，打开时清空过滤词", () => {
    const palette = useCommandPalette();
    expect(palette.open.value).toBe(false);

    pressKey("k", { metaKey: true });
    expect(palette.open.value).toBe(true);
    expect(palette.query.value).toBe("");

    palette.query.value = "报告";
    pressKey("k", { ctrlKey: true });
    expect(palette.open.value).toBe(false);

    pressKey("k", { metaKey: true });
    expect(palette.open.value).toBe(true);
    expect(palette.query.value).toBe("");
    palette.closePalette();
  });

  it("空关键词返回全量命令（阶段 + 菜单命令）", () => {
    const palette = useCommandPalette();
    palette.openPalette();
    const labels = palette.filtered.value.map((command) => `${command.group} ${command.label}`);
    expect(labels).toContain("分析阶段 主页");
    expect(labels).toContain("文件 保存");
    expect(labels).toContain("工具 关闭虚拟机");
    expect(labels).toContain("结果 导出当前场为 CSV");
  });

  it("关键词按分组与标签过滤，无匹配返回空", () => {
    const palette = useCommandPalette();
    palette.openPalette();

    palette.query.value = "虚拟机";
    expect(palette.filtered.value.length).toBe(3); // 启动 / 进入 Shell / 关闭

    palette.query.value = "分析阶段 求解";
    expect(palette.filtered.value.map((command) => command.label)).toEqual(["求解"]);

    palette.query.value = "不存在的命令";
    expect(palette.filtered.value).toEqual([]);
  });

  it("方向键移动高亮并夹在边界内，回车执行高亮项", () => {
    const app = useAppStore();
    const palette = useCommandPalette();
    palette.openPalette();

    palette.moveActive(-1); // 已在首条，不动
    expect(palette.activeIndex.value).toBe(0);
    palette.moveActive(1);
    palette.moveActive(1);
    expect(palette.activeIndex.value).toBe(2);

    pressKey("ArrowUp");
    expect(palette.activeIndex.value).toBe(1);

    // 高亮第 2 条 = 分析阶段「几何」
    pressKey("Enter");
    expect(app.stage).toBe("geometry");
    expect(palette.open.value).toBe(false);
  });

  it("回车时无候选则不动作", () => {
    const palette = useCommandPalette();
    palette.openPalette();
    palette.query.value = "不存在";
    pressKey("ArrowDown");
    pressKey("Enter");
    expect(palette.open.value).toBe(true);
  });

  it("Esc 关闭面板", () => {
    const palette = useCommandPalette();
    palette.openPalette();
    pressKey("Escape");
    expect(palette.open.value).toBe(false);
  });

  it("面板关闭时普通按键直接早退", () => {
    const palette = useCommandPalette();
    expect(palette.open.value).toBe(false);
    const event = new KeyboardEvent("keydown", { key: "a", cancelable: true });
    window.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(false);
    expect(palette.open.value).toBe(false);
  });

  it("面板打开时其余按键不拦截（无 preventDefault）", () => {
    const palette = useCommandPalette();
    palette.openPalette();
    const event = new KeyboardEvent("keydown", { key: "a", cancelable: true });
    window.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(false);
  });
});

describe("CommandPalette 组件", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    useCommandPalette().closePalette();
  });

  it("打开后渲染搜索框与命令列表，点击项执行并关闭", async () => {
    const app = useAppStore();
    const wrapper = mount(CommandPalette, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    const palette = useCommandPalette();
    palette.openPalette();
    await wrapper.vm.$nextTick();

    // Teleport 到 body，须经 document 查询
    const input = document.querySelector("input");
    expect(input).not.toBeNull();
    expect(input?.getAttribute("placeholder")).toBe("搜索命令…");

    const items = [...document.querySelectorAll("ul button")];
    expect(items.length).toBe(palette.filtered.value.length);

    // 点击「求解」阶段命令
    const solve = items.find((item) =>
      (item as HTMLElement).textContent?.replace(/\s/g, "").includes("求解"),
    )!;
    (solve as HTMLElement).click();
    await wrapper.vm.$nextTick();
    expect(app.stage).toBe("solve");
    expect(palette.open.value).toBe(false);
    wrapper.unmount();
  });

  it("无匹配时显示空态提示", async () => {
    const wrapper = mount(CommandPalette, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    const palette = useCommandPalette();
    palette.openPalette();
    palette.query.value = "查无此命令";
    await wrapper.vm.$nextTick();
    expect(document.body.textContent).toContain("没有匹配的命令");
    wrapper.unmount();
  });
});
