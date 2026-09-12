import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createFieldAnimation } from "../../../src-web/utils/animation";

describe("createFieldAnimation（时间步节拍器）", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("按间隔逐帧推进并对时间步序列循环回绕", async () => {
    const animation = createFieldAnimation(400);
    const loaded: string[] = [];
    animation.start(["a", "b"], async (dirName) => {
      loaded.push(dirName);
    });
    await vi.advanceTimersByTimeAsync(400);
    await vi.advanceTimersByTimeAsync(400);
    await vi.advanceTimersByTimeAsync(400);
    expect(loaded).toEqual(["a", "b", "a"]);
    expect(animation.advanced).toBe(3);
    animation.stop();
  });

  it("背压：上一步未完成时跳过本拍，完成后恢复推进", async () => {
    const animation = createFieldAnimation(400);
    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    const loaded: string[] = [];
    animation.start(["a", "b", "c"], async (dirName) => {
      loaded.push(dirName);
      await gate;
    });
    await vi.advanceTimersByTimeAsync(400);
    expect(loaded).toEqual(["a"]);
    expect(animation.inFlight).toBe(true);
    // 三拍都在 a 完成前到达：全部被背压跳过
    await vi.advanceTimersByTimeAsync(1200);
    expect(loaded).toEqual(["a"]);
    expect(animation.advanced).toBe(1);
    release();
    await vi.advanceTimersByTimeAsync(400);
    expect(loaded).toEqual(["a", "b"]);
    animation.stop();
  });

  it("stop 后不再推进；重复 start 先停旧节奏", async () => {
    const animation = createFieldAnimation(400);
    const loaded: string[] = [];
    animation.start(["a"], async (dirName) => {
      loaded.push(dirName);
    });
    await vi.advanceTimersByTimeAsync(400);
    animation.stop();
    await vi.advanceTimersByTimeAsync(1200);
    expect(loaded).toEqual(["a"]);
    // 空转 stop 不抛错
    animation.stop();
    // 重新 start：节奏重启且从头循环
    animation.start(["x", "y"], async (dirName) => {
      loaded.push(dirName);
    });
    await vi.advanceTimersByTimeAsync(400);
    expect(loaded).toEqual(["a", "x"]);
    animation.stop();
  });
});
