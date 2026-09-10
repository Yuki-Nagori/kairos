import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { IpcUnavailableError } from "../../../src-web/utils/ipc";
import { useAppStore } from "../../../src-web/stores/app";

describe("app store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
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
    });

    it("accepts plain string failures", () => {
      const app = useAppStore();
      app.setError("字符串错误");
      expect(app.error?.message).toBe("字符串错误");
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
});
