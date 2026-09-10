import { describe, expect, it } from "vitest";
import { useLayout } from "../../../src-web/composables/useLayout";

// 模块级状态 + 惰性初始化在整个文件内只走一次：
// 第一个用例必须在预置 localStorage 后首次调用，覆盖「启动恢复」路径。
describe("useLayout", () => {
  it("首次调用恢复持久化的折叠偏好", () => {
    localStorage.setItem("kairos-layout:left", "1");
    const { leftCollapsed, rightCollapsed } = useLayout();
    expect(leftCollapsed.value).toBe(true);
    expect(rightCollapsed.value).toBe(false);
  });

  it("切换左列折叠并持久化到 localStorage", () => {
    const { leftCollapsed, toggleLeft } = useLayout();
    toggleLeft();
    expect(leftCollapsed.value).toBe(false);
    expect(localStorage.getItem("kairos-layout:left")).toBe("0");

    toggleLeft();
    expect(leftCollapsed.value).toBe(true);
    expect(localStorage.getItem("kairos-layout:left")).toBe("1");
  });

  it("右列折叠独立于左列", () => {
    const { leftCollapsed, rightCollapsed, toggleLeft, toggleRight } = useLayout();
    toggleLeft();
    toggleRight();
    expect(rightCollapsed.value).toBe(true);
    expect(leftCollapsed.value).toBe(false);
    expect(localStorage.getItem("kairos-layout:right")).toBe("1");

    // 再切一次恢复展开态，持久化写回 "0"
    toggleRight();
    expect(rightCollapsed.value).toBe(false);
    expect(localStorage.getItem("kairos-layout:right")).toBe("0");
  });
});
