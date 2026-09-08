import { describe, expect, it } from "vitest";
import { isTauriRuntime } from "../../../src-web/lib/environment";

describe("isTauriRuntime", () => {
  it("无 __TAURI_INTERNALS__ 时为 false", () => {
    expect(isTauriRuntime()).toBe(false);
  });

  it("存在 __TAURI_INTERNALS__ 时为 true", () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    expect(isTauriRuntime()).toBe(true);
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });
});
