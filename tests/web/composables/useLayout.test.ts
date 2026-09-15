import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useLayout } from "../../../src-web/composables/useLayout";
import { useAppStore } from "../../../src-web/stores/app";

// 模块级状态 + 惰性初始化在整个文件内只走一次：
// 第一个用例必须在预置 localStorage 后首次调用，覆盖「启动恢复」路径。
describe("useLayout", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("首次调用恢复持久化的折叠偏好", () => {
    localStorage.setItem("kairos:layout:left", "true");
    const { leftCollapsed, rightCollapsed } = useLayout();
    expect(leftCollapsed.value).toBe(true);
    expect(rightCollapsed.value).toBe(false);
  });

  it("切换左列折叠并持久化到 localStorage", () => {
    const { leftCollapsed, toggleLeft } = useLayout();
    toggleLeft();
    expect(leftCollapsed.value).toBe(false);
    expect(localStorage.getItem("kairos:layout:left")).toBe("false");

    toggleLeft();
    expect(leftCollapsed.value).toBe(true);
    expect(localStorage.getItem("kairos:layout:left")).toBe("true");
  });

  it("右列折叠独立于左列", () => {
    const { leftCollapsed, rightCollapsed, toggleLeft, toggleRight } = useLayout();
    toggleLeft();
    toggleRight();
    expect(rightCollapsed.value).toBe(true);
    expect(leftCollapsed.value).toBe(false);
    expect(localStorage.getItem("kairos:layout:right")).toBe("true");

    // 再切一次恢复展开态，持久化写回 "0"
    toggleRight();
    expect(rightCollapsed.value).toBe(false);
    expect(localStorage.getItem("kairos:layout:right")).toBe("false");
  });

  it("三列列宽随左右折叠组合切换（四种组合各自的类名）", () => {
    const { leftCollapsed, rightCollapsed, toggleLeft, toggleRight, gridClass } = useLayout();
    // 上两个用例留下的是「左折 / 右展」起点，逐个组合走一遍
    const expected: Array<[boolean, boolean, string]> = [
      [true, false, "grid-cols-[0px_minmax(0,1fr)_320px]"],
      [true, true, "grid-cols-[0px_minmax(0,1fr)_0px]"],
      [false, true, "grid-cols-[280px_minmax(0,1fr)_0px]"],
      [false, false, "grid-cols-[280px_minmax(0,1fr)_320px]"],
    ];
    for (const [left, right, cls] of expected) {
      if (leftCollapsed.value !== left) {
        toggleLeft();
      }
      if (rightCollapsed.value !== right) {
        toggleRight();
      }
      expect(gridClass.value).toBe(cls);
    }
  });

  it("面板阶段可见性按当前分析阶段判断", () => {
    const { stageVisible } = useLayout();
    const app = useAppStore();
    app.stage = "mesh";
    expect(stageVisible(["home", "geometry", "mesh"])).toBe(true);
    expect(stageVisible(["home", "results"])).toBe(false);
  });
});
