import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { appStore, initialAppState } from "../../../src-web/state";
import type { SystemInfo } from "../../../src-web/types";
import { createStatusBar } from "../../../src-web/components/app-header";

const info: SystemInfo = { name: "kairos", version: "0.1.0", os: "macos" };

function statusLine(bar: HTMLElement): string {
  const status = bar.firstElementChild;
  if (!status) {
    throw new Error("status line not found");
  }
  return status.textContent ?? "";
}

describe("createAppHeader status precedence", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
  });

  afterEach(() => {
    appStore.set(initialAppState);
  });

  it("shows version and platform when idle", () => {
    appStore.set({ info });
    expect(statusLine(createStatusBar())).toContain("v0.1.0");
  });

  it("busy takes precedence over info", () => {
    appStore.set({ info, busy: "正在求解…" });
    expect(statusLine(createStatusBar())).toBe("正在求解…");
  });

  it("error takes precedence over busy and info", () => {
    appStore.set({
      info,
      busy: "正在求解…",
      error: { message: "迭代不收敛", info: false },
    });
    expect(statusLine(createStatusBar())).toBe("迭代不收敛");
  });
});

describe("status bar shell toggle", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
  });

  afterEach(() => {
    appStore.set(initialAppState);
  });

  it("toggles the vm panel visibility from the shell button", () => {
    const bar = createStatusBar();
    const button = [...bar.querySelectorAll("button")].at(-1);
    if (!button) {
      throw new Error("shell button not found");
    }
    // 文字提示在左，shell 图标（currentColor SVG）在右
    expect(button.textContent).toContain("Shell 环境");
    expect(button.querySelector("svg")).not.toBeNull();
    expect(appStore.get().vmPanelVisible).toBe(false);

    button.click();
    expect(appStore.get().vmPanelVisible).toBe(true);
    expect(button.textContent).toContain("点击收起");

    button.click();
    expect(appStore.get().vmPanelVisible).toBe(false);
  });
});
