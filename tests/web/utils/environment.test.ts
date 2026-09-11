import { describe, expect, it } from "vitest";
import { detectPlatform, isTauriRuntime } from "../../../src-web/utils/environment";

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

describe("detectPlatform", () => {
  it("Macintosh / Mac OS X UA 归为 macos", () => {
    expect(detectPlatform("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")).toBe("macos");
  });

  it("Windows UA 归为 windows", () => {
    expect(detectPlatform("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")).toBe("windows");
  });

  it("其余 UA 归为 linux", () => {
    expect(detectPlatform("Mozilla/5.0 (X11; Linux x86_64)")).toBe("linux");
  });
});
