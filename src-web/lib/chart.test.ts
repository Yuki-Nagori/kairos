import { describe, expect, it } from "vitest";
import { downsampleSeries, toCsv } from "./chart";

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
