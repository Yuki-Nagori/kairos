import { afterEach, describe, expect, it, vi } from "vitest";
import { drawLineChart, downsampleSeries, toCsv } from "./chart";

describe("downsampleSeries", () => {
  it("returns a copy when small enough", () => {
    const values = [1, 2, 3];
    expect(downsampleSeries(values, 10)).toEqual([1, 2, 3]);
    expect(downsampleSeries(values, 10)).not.toBe(values);
  });

  it("keeps one point per bucket roughly", () => {
    const values = Array.from({ length: 100 }, (_, i) => i);
    const sampled = downsampleSeries(values, 10);
    // min-max 抽样：每桶 2 点
    expect(sampled.length).toBe(20);
    // 首桶 min 0
    expect(sampled[0]).toBe(0);
    // 末桶 max 99
    expect(sampled[sampled.length - 1]).toBe(99);
  });

  it("preserves extremes inside each bucket", () => {
    // 桶内 [5, -100, 7]：min-max 应保留 -100 与 7，而非漏掉尖峰
    const values = [0, 5, -100, 7, 0, 0, 0, 0];
    const sampled = downsampleSeries(values, 2);
    expect(Math.min(...sampled)).toBe(-100);
    expect(Math.max(...sampled)).toBe(7);
  });

  it("handles empty input", () => {
    expect(downsampleSeries([], 10)).toEqual([]);
  });
});

describe("toCsv", () => {
  it("joins headers and rows", () => {
    const csv = toCsv(
      ["node", "T"],
      [
        [1, 300],
        [2, 301.5],
      ],
    );
    expect(csv).toBe("node,T\n1,300\n2,301.5");
  });
});

/** 录制调用的假 2D 上下文：属性可写，方法调用记入 calls。 */
function fakeCtx(): CanvasRenderingContext2D & { calls: string[] } {
  const calls: string[] = [];
  return new Proxy(
    {},
    {
      get(_target, prop) {
        if (prop === "calls") {
          return calls;
        }
        return (...args: unknown[]) => {
          void args;
          calls.push(String(prop));
        };
      },
      set() {
        return true;
      },
    },
  ) as CanvasRenderingContext2D & { calls: string[] };
}

describe("drawLineChart", () => {
  it("空序列只铺背景，返回 0", () => {
    const ctx = fakeCtx();
    const drawn = drawLineChart(ctx, [], { width: 200, height: 100 });
    expect(drawn).toBe(0);
    expect(ctx.calls).toContain("fillRect");
    expect(ctx.calls).not.toContain("stroke");
  });

  it("绘制曲线：描边、网格与坐标轴文字都发生", () => {
    const ctx = fakeCtx();
    const values = Array.from({ length: 50 }, (_, i) => Math.sin(i / 5));
    const drawn = drawLineChart(ctx, [{ values, color: "#34d399", label: "p" }], {
      width: 400,
      height: 200,
      xLabel: "x",
      yLabel: "y",
    });
    expect(drawn).toBeGreaterThan(0);
    expect(ctx.calls).toContain("stroke");
    expect(ctx.calls).toContain("fillText");
  });

  it("cssVar 缺失时使用回退色（不抛错）", () => {
    const ctx = fakeCtx();
    expect(() =>
      drawLineChart(ctx, [{ values: [1, 2], color: "#fff", label: "" }], {
        width: 100,
        height: 80,
      }),
    ).not.toThrow();
  });
});

describe("drawLineChart 与 downsampleSeries 的边界", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("downsampleSeries：值数不超过桶数两倍时原样返回", () => {
    // 早退分支：值太密不需要降采样
    const values = [1, 2];
    expect(downsampleSeries(values, 3)).toEqual([1, 2]);
  });

  it("drawLineChart：命中 CSS 变量时使用变量值", () => {
    vi.stubGlobal("getComputedStyle", () => ({
      getPropertyValue: (name: string) => (name === "--c-bg-input" ? "#abcdef" : ""),
    }));
    const ctx = fakeCtx();
    const drawn = drawLineChart(ctx, [{ values: [1, 2], color: "#fff", label: "" }], {
      width: 100,
      height: 80,
    });
    expect(drawn).toBeGreaterThan(0);
  });

  it("drawLineChart：cssVar 不可用时用回退色不抛错", () => {
    vi.stubGlobal("getComputedStyle", undefined);
    const ctx = fakeCtx();
    expect(() =>
      drawLineChart(ctx, [{ values: [1, 2], color: "#fff", label: "" }], {
        width: 100,
        height: 80,
      }),
    ).not.toThrow();
  });

  it("drawLineChart：平坦序列自动扩展示宽范围", () => {
    const ctx = fakeCtx();
    const drawn = drawLineChart(ctx, [{ values: [5, 5, 5], color: "#fff", label: "" }], {
      width: 200,
      height: 100,
    });
    expect(drawn).toBeGreaterThan(0);
  });

  it("drawLineChart：跳过空序列项", () => {
    const ctx = fakeCtx();
    const drawn = drawLineChart(
      ctx,
      [
        { values: [], color: "#fff", label: "空" },
        { values: [1, 2, 3], color: "#fff", label: "有" },
      ],
      { width: 200, height: 100 },
    );
    expect(drawn).toBeGreaterThan(0);
  });
});
