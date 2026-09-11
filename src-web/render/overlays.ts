/**
 * 视口线段叠加层的纯数据构造：把研究的浇注系统 / 冷却水路定义
 * 换算为渲染器可上传的线段坐标与配色（与 WebGL 无关，可独立单测）。
 */
import type { CoolingChannel, RunnerElement } from "../types";

/** 线段叠加层：成对端点的扁平坐标 + RGB 颜色（0..1）。 */
export interface OverlayLayer {
  /** 成对端点坐标（长度 = 6 × 线段数）。 */
  positions: Float32Array;
  /** RGB 颜色（0..1）。 */
  color: [number, number, number];
}

/** 叠加层 id：与渲染器 uploadOverlay / 外壳图层状态的键一致。 */
export const OVERLAY_IDS = {
  gates: "gates",
  runners: "runners",
  cooling: "cooling",
} as const;

/** 叠加层配色：浇口红、流道琥珀、冷却水路蓝（深色视口下高对比）。 */
const GATE_COLOR: [number, number, number] = [0.95, 0.32, 0.3];
const RUNNER_COLOR: [number, number, number] = [0.95, 0.62, 0.12];
const COOLING_COLOR: [number, number, number] = [0.3, 0.6, 0.95];

/** 端点对压平成线段坐标（长度 = 6 × 段数）。 */
function flattenSegments(
  elements: { start: [number, number, number]; end: [number, number, number] }[],
): Float32Array {
  const positions = new Float32Array(elements.length * 6);
  elements.forEach((element, index) => {
    positions.set(element.start, index * 6);
    positions.set(element.end, index * 6 + 3);
  });
  return positions;
}

/** 研究中参与叠加层构造的几何定义子集。 */
interface OverlaySource {
  runnerElements: RunnerElement[];
  coolingChannels: CoolingChannel[];
}

/** 按研究定义构造三层叠加层：浇口 / 流道按 kind 分组，冷却水路独立。 */
export function buildOverlayLayers(source: OverlaySource): {
  gates: OverlayLayer;
  runners: OverlayLayer;
  cooling: OverlayLayer;
} {
  return {
    gates: {
      positions: flattenSegments(
        source.runnerElements.filter((element) => element.kind === "gate"),
      ),
      color: GATE_COLOR,
    },
    runners: {
      positions: flattenSegments(
        source.runnerElements.filter((element) => element.kind === "runner"),
      ),
      color: RUNNER_COLOR,
    },
    cooling: {
      positions: flattenSegments(source.coolingChannels),
      color: COOLING_COLOR,
    },
  };
}
