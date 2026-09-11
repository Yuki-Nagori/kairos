import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { enableAutoUnmount, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import type { DOMWrapper, VueWrapper } from "@vue/test-utils";
import MenuBar from "../../../src-web/components/menu-bar/MenuBar.vue";
import { useAboutDialog } from "../../../src-web/components/menu-bar/useAboutDialog";
import { useCommandPalette } from "../../../src-web/components/command-palette/useCommandPalette";
import { useAppStore } from "../../../src-web/stores/app";
import { useProjectStore } from "../../../src-web/stores/project";

const GROUP_LABELS = ["文件", "视图", "工具", "结果", "报告", "帮助"];

// MenuBar 监听 window 事件且对话框 Teleport 到 body，不卸载会跨用例残留。
enableAutoUnmount(afterEach);

/** 打开指定下标的一级菜单（点击其按钮）。 */
async function openMenu(wrapper: VueWrapper, index: number) {
  await wrapper.findAll("nav > div > button")[index]!.trigger("click");
}

/** 取指定分组的下拉容器（每个分组各有一个，v-show 控制显隐）。 */
function dropdown(wrapper: VueWrapper, index: number): DOMWrapper<Element> {
  return wrapper.findAll("nav .absolute")[index]!;
}

/** v-show 显隐断言走内联 display（happy-dom 下 isVisible 不反映 v-show）。 */
function visible(el: DOMWrapper<Element>): boolean {
  return (el.element as HTMLElement).style.display !== "none";
}

describe("MenuBar（Windows/Linux 自绘标题栏）", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    // 标题栏菜单仅非 macOS 平台渲染
    vi.stubGlobal("navigator", { userAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64)" });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    useAboutDialog().hideAbout();
  });

  it("渲染品牌、六个一级菜单与搜索入口", () => {
    const wrapper = mount(MenuBar, { global: { plugins: [pinia] } });
    expect(wrapper.find("h1").text()).toBe("Kairos");
    const labels = wrapper.findAll("nav > div > button").map((button) => button.text());
    expect(labels).toEqual(GROUP_LABELS);
    expect(wrapper.text()).toContain("搜索命令…");
  });

  it("点击展开下拉菜单，再次点击收起", async () => {
    const wrapper = mount(MenuBar, { global: { plugins: [pinia] } });
    await openMenu(wrapper, 0);
    expect(visible(dropdown(wrapper, 0))).toBe(true);
    expect(dropdown(wrapper, 0).text()).toContain("新建项目");

    await openMenu(wrapper, 0);
    expect(visible(dropdown(wrapper, 0))).toBe(false);
  });

  it("展开期间悬停其他一级菜单直接切换", async () => {
    const wrapper = mount(MenuBar, { global: { plugins: [pinia] } });
    await openMenu(wrapper, 0);
    await wrapper.findAll("nav > div > button")[2]!.trigger("mouseenter");
    expect(visible(dropdown(wrapper, 0))).toBe(false);
    expect(visible(dropdown(wrapper, 2))).toBe(true);
    expect(dropdown(wrapper, 2).text()).toContain("启动虚拟机");
  });

  it("未展开时悬停不打开菜单", async () => {
    const wrapper = mount(MenuBar, { global: { plugins: [pinia] } });
    await wrapper.findAll("nav > div > button")[0]!.trigger("mouseenter");
    expect(visible(dropdown(wrapper, 0))).toBe(false);
  });

  it("点击菜单项执行动作并收起菜单", async () => {
    const project = useProjectStore();
    const saveSpy = vi.spyOn(project, "saveProject").mockReturnValue(Promise.resolve());
    const wrapper = mount(MenuBar, { global: { plugins: [pinia] } });
    await openMenu(wrapper, 0);

    const item = dropdown(wrapper, 0).findAll("button")[2]!; // 保存
    expect(item.text()).toContain("保存");
    expect(item.text()).toContain("Ctrl+S");
    await item.trigger("click");

    expect(saveSpy).toHaveBeenCalledOnce();
    expect(visible(dropdown(wrapper, 0))).toBe(false);
  });

  it("帮助菜单打开关于对话框并显示版本，可关闭", async () => {
    const app = useAppStore();
    app.info = { name: "kairos", version: "0.2.0", os: "windows" };
    const wrapper = mount(MenuBar, { global: { plugins: [pinia] } });

    const helpIndex = GROUP_LABELS.length - 1;
    await openMenu(wrapper, helpIndex);
    await dropdown(wrapper, helpIndex).find("button").trigger("click");

    const dialogTitle = document.querySelector("h2");
    expect(dialogTitle?.textContent).toBe("Kairos");
    expect(document.body.textContent).toContain("v0.2.0 · windows");

    // 关于对话框无图标占位：标题行直接是文字，无 img
    expect(document.querySelectorAll(".fixed img").length).toBe(0);

    // 在对话框浮层内找关闭按钮（导航里还有「关闭虚拟机」菜单项，不能全局搜）
    const closeButton = [...document.querySelectorAll<HTMLElement>(".fixed button")].find(
      (button) => button.textContent?.includes("关闭"),
    )!;
    closeButton.click();
    await wrapper.vm.$nextTick();
    expect(document.querySelector("h2")?.textContent ?? null).toBeNull();
  });

  it("Esc 关闭展开的菜单，其余按键不影响", async () => {
    const wrapper = mount(MenuBar, { global: { plugins: [pinia] } });
    await openMenu(wrapper, 0);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "a" }));
    await wrapper.vm.$nextTick();
    expect(visible(dropdown(wrapper, 0))).toBe(true);

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await wrapper.vm.$nextTick();
    expect(visible(dropdown(wrapper, 0))).toBe(false);
  });

  it("菜单区域外的按下关闭菜单，区域内不关闭", async () => {
    const wrapper = mount(MenuBar, { global: { plugins: [pinia] } });
    await openMenu(wrapper, 0);

    document.body.dispatchEvent(new Event("pointerdown", { bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(visible(dropdown(wrapper, 0))).toBe(false);

    await openMenu(wrapper, 0);
    await wrapper.find("nav").trigger("pointerdown");
    await wrapper.vm.$nextTick();
    expect(visible(dropdown(wrapper, 0))).toBe(true);
  });

  it("无菜单展开时外部按下不产生任何影响", async () => {
    const wrapper = mount(MenuBar, { global: { plugins: [pinia] } });
    document.body.dispatchEvent(new Event("pointerdown", { bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(visible(dropdown(wrapper, 0))).toBe(false);
  });

  it("按下目标非元素（如 window）时不误关菜单", async () => {
    const wrapper = mount(MenuBar, { global: { plugins: [pinia] } });
    await openMenu(wrapper, 0);
    window.dispatchEvent(new Event("pointerdown"));
    await wrapper.vm.$nextTick();
    expect(visible(dropdown(wrapper, 0))).toBe(true);
  });

  it("点击搜索框打开命令面板", async () => {
    const wrapper = mount(MenuBar, { global: { plugins: [pinia] } });
    const search = wrapper.findAll("button").at(-1)!;
    await search.trigger("click");
    expect(useCommandPalette().open.value).toBe(true);
    useCommandPalette().closePalette();
  });

  it("macOS 桌面（系统菜单栏承载菜单）不渲染 web 菜单", () => {
    vi.stubGlobal("navigator", { userAgent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)" });
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    const wrapper = mount(MenuBar, { global: { plugins: [pinia] } });
    expect(wrapper.find("nav").exists()).toBe(false);
    expect(wrapper.text()).toContain("搜索命令…");
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it("macOS 浏览器预览：系统菜单栏属于浏览器，仍渲染 web 菜单", () => {
    vi.stubGlobal("navigator", { userAgent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)" });
    const wrapper = mount(MenuBar, { global: { plugins: [pinia] } });
    expect(wrapper.find("nav").exists()).toBe(true);
  });

  it("三端均不自绘窗口控制按钮（macOS 红绿灯 / Win-Linux 插件内嵌控制）", () => {
    for (const userAgent of [
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
    ]) {
      vi.stubGlobal("navigator", { userAgent });
      const wrapper = mount(MenuBar, { global: { plugins: [pinia] } });
      const labels = wrapper.findAll("button").map((button) => button.text());
      expect(labels).not.toContain("─");
      expect(labels).not.toContain("▢");
      expect(labels).not.toContain("✕");
    }
  });
});
