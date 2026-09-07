import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { IpcUnavailableError } from "./lib/ipc";
import { appStore, bootstrap, initialAppState, setError } from "./state";
import { getSystemInfo } from "./services/system";

vi.mock("./services/system", () => ({ getSystemInfo: vi.fn() }));

describe("setError", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
  });

  afterEach(() => {
    appStore.set(initialAppState);
    vi.mocked(getSystemInfo).mockReset();
  });

  it("classifies IPC unavailability as an info hint", () => {
    setError(new IpcUnavailableError());
    expect(appStore.get().error?.info).toBe(true);
    expect(appStore.get().error?.message).toContain("Tauri runtime");
  });

  it("classifies other errors as real failures", () => {
    setError(new Error("boom"));
    expect(appStore.get().error?.info).toBe(false);
    expect(appStore.get().error?.message).toBe("boom");
  });

  it("accepts plain string failures", () => {
    setError("字符串错误");
    expect(appStore.get().error?.message).toBe("字符串错误");
  });
});

describe("bootstrap", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
  });

  afterEach(() => {
    appStore.set(initialAppState);
    vi.mocked(getSystemInfo).mockReset();
  });

  it("stores app info when IPC responds", async () => {
    vi.mocked(getSystemInfo).mockResolvedValue({
      name: "kairos",
      version: "0.1.0",
      os: "macos",
    });
    await bootstrap();
    expect(appStore.get().info?.name).toBe("kairos");
    expect(appStore.get().error).toBeNull();
  });

  it("surfaces IPC unavailability as an info hint", async () => {
    vi.mocked(getSystemInfo).mockRejectedValue(new IpcUnavailableError());
    await bootstrap();
    expect(appStore.get().info).toBeNull();
    expect(appStore.get().error?.info).toBe(true);
  });
});
