import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { CommandError, IpcUnavailableError } from "../../../src-web/utils/ipc";
import { useAppStore } from "../../../src-web/stores/app";

describe("app store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("setError", () => {
    it("classifies IPC unavailability as an info hint", () => {
      const app = useAppStore();
      app.setError(new IpcUnavailableError());
      expect(app.error?.info).toBe(true);
      expect(app.error?.message).toContain("Tauri runtime");
    });

    it("classifies other errors as real failures", () => {
      const app = useAppStore();
      app.setError(new Error("boom"));
      expect(app.error?.info).toBe(false);
      expect(app.error?.message).toBe("boom");
      expect(app.error?.code).toBeNull();
    });

    it("keeps the contract code of CommandError for code-based branching", () => {
      const app = useAppStore();
      app.setError(new CommandError("io", "读取文件失败"));
      expect(app.error?.code).toBe("io");
      expect(app.error?.info).toBe(false);
    });

    it("accepts plain string failures", () => {
      const app = useAppStore();
      app.setError("字符串错误");
      expect(app.error?.message).toBe("字符串错误");
    });

    it("auto-clears an info hint after 8 seconds", () => {
      vi.useFakeTimers();
      const app = useAppStore();
      app.setError(new IpcUnavailableError());
      vi.advanceTimersByTime(7999);
      expect(app.error).not.toBeNull();
      vi.advanceTimersByTime(1);
      expect(app.error).toBeNull();
    });

    it("clears the previous hint timer when a new error arrives", () => {
      vi.useFakeTimers();
      const app = useAppStore();
      app.setError(new IpcUnavailableError());
      vi.advanceTimersByTime(4000);
      app.setError(new IpcUnavailableError());
      vi.advanceTimersByTime(4000);
      // t=8000：若第一个 timer 未被清除，error 此时已被它清空。
      expect(app.error).not.toBeNull();
      vi.advanceTimersByTime(4000);
      // t=12000：第二个 timer（8 秒后）到期。
      expect(app.error).toBeNull();
    });

    it("keeps a real failure that replaced a pending hint timer", () => {
      vi.useFakeTimers();
      const app = useAppStore();
      app.setError(new IpcUnavailableError());
      app.setError("真实失败");
      vi.advanceTimersByTime(8000);
      expect(app.error?.message).toBe("真实失败");
    });

    it("does not resurrect an error cleared by beginBusy when the timer fires", () => {
      vi.useFakeTimers();
      const app = useAppStore();
      app.setError(new IpcUnavailableError());
      // beginBusy 清掉 error 但不清 timer：timer 触发时 message 不匹配，不应写回。
      app.beginBusy("正在求解…");
      vi.advanceTimersByTime(8000);
      expect(app.error).toBeNull();
      expect(app.busy).toBe("正在求解…");
    });
  });

  describe("beginBusy / endBusy", () => {
    it("beginBusy enters busy and clears the last error; endBusy resets", () => {
      const app = useAppStore();
      app.setError("上次错误");
      app.beginBusy("正在求解…");
      expect(app.busy).toBe("正在求解…");
      expect(app.error).toBeNull();
      app.endBusy();
      expect(app.busy).toBeNull();
    });
  });

  describe("withBusy", () => {
    it("wraps a successful run: busy lifecycle plus transparent return value", async () => {
      const app = useAppStore();
      const result = await app.withBusy("正在计算…", async () => 42);
      expect(result).toBe(42);
      expect(app.busy).toBeNull();
      expect(app.error).toBeNull();
    });

    it("routes failures into the global error state and returns undefined", async () => {
      const app = useAppStore();
      const result = await app.withBusy("正在计算…", async () => {
        throw new Error("计算失败");
      });
      expect(result).toBeUndefined();
      expect(app.busy).toBeNull();
      expect(app.error?.message).toBe("计算失败");
    });

    it("clears the previous error when entering busy (beginBusy semantics)", async () => {
      const app = useAppStore();
      app.setError("上次的错误");
      await app.withBusy("正在计算…", async () => undefined);
      // withBusy 结束后错误仍为空（过程中无新错误）
      expect(app.error).toBeNull();
    });
  });
});
