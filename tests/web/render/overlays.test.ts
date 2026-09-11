import { describe, expect, it } from "vitest";
import { buildOverlayLayers, OVERLAY_IDS } from "../../../src-web/render/overlays";
import type { CoolingChannel, RunnerElement } from "../../../src-web/types";

const gate: RunnerElement = {
  id: "g1",
  kind: "gate",
  diameterMm: 2,
  start: [0, 0, 0],
  end: [1, 2, 3],
};
const runner: RunnerElement = {
  id: "r1",
  kind: "runner",
  diameterMm: 6,
  start: [1, 0, 0],
  end: [5, 0, 0],
};
const channel: CoolingChannel = {
  id: "c1",
  diameterMm: 8,
  start: [0, 5, 0],
  end: [5, 5, 0],
  inletTempC: 25,
};

describe("buildOverlayLayers", () => {
  it("按 kind 分组浇口 / 流道，冷却水路独立成层", () => {
    const layers = buildOverlayLayers({
      runnerElements: [gate, runner],
      coolingChannels: [channel],
    });
    // 每段压平为 6 个坐标
    expect(layers.gates.positions.length).toBe(6);
    expect(layers.runners.positions.length).toBe(6);
    expect(layers.cooling.positions.length).toBe(6);
  });

  it("线段坐标按起点 → 终点成对压平", () => {
    const { positions } = buildOverlayLayers({
      runnerElements: [gate],
      coolingChannels: [],
    }).gates;
    expect(Array.from(positions.slice(0, 3))).toEqual([0, 0, 0]);
    expect(Array.from(positions.slice(3, 6))).toEqual([1, 2, 3]);
  });

  it("三层配色符合约定：浇口红 / 流道琥珀 / 冷却蓝", () => {
    const layers = buildOverlayLayers({
      runnerElements: [gate, runner],
      coolingChannels: [channel],
    });
    expect(layers.gates.color).toEqual([0.95, 0.32, 0.3]);
    expect(layers.runners.color).toEqual([0.95, 0.62, 0.12]);
    expect(layers.cooling.color).toEqual([0.3, 0.6, 0.95]);
  });

  it("空研究构造零长度坐标（渲染器将其视为清除该层）", () => {
    const layers = buildOverlayLayers({ runnerElements: [], coolingChannels: [] });
    expect(layers.gates.positions.length).toBe(0);
    expect(layers.runners.positions.length).toBe(0);
    expect(layers.cooling.positions.length).toBe(0);
  });

  it("叠加层 id 与图层状态键一致", () => {
    expect(Object.values(OVERLAY_IDS)).toEqual(["gates", "runners", "cooling"]);
  });
});
