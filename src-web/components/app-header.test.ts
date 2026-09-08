import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { appStore, initialAppState } from "../state";
import type { SystemInfo } from "../types";
import { createStatusBar } from "./app-header";

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
