/**
 * 视口显示状态：图层可见性跨面板共享（层管理面板写、视口渲染器读）。
 * 只存「意图」——把状态应用到 WebGL 渲染器是视口 composable 的职责。
 */
import { defineStore } from "pinia";

export interface LayerState {
  /** 制品表面网格。 */
  mesh: boolean;
  /** 浇口线段。 */
  gates: boolean;
  /** 流道线段。 */
  runners: boolean;
  /** 冷却水路线段。 */
  cooling: boolean;
}

const DEFAULT_VISIBLE: LayerState = {
  mesh: true,
  gates: true,
  runners: true,
  cooling: true,
};

export type ViewportLayout = "single" | "quad";

export const useViewportStore = defineStore("viewport", {
  state: () => ({
    layers: { ...DEFAULT_VISIBLE } as LayerState,
    /** 视口布局：单视口 / 四分格（多视口联动）。 */
    layout: "single" as ViewportLayout,
  }),
  actions: {
    /** 切换图层可见性。 */
    setLayerVisible(id: keyof LayerState, visible: boolean): void {
      this.layers = { ...this.layers, [id]: visible };
    },
    /** 恢复默认全开（切换研究 / 重置视图时调用）。 */
    resetLayers(): void {
      this.layers = { ...DEFAULT_VISIBLE };
    },
    /** 切换视口布局（单视口 / 四分格）。 */
    setLayout(layout: ViewportLayout): void {
      this.layout = layout;
    },
  },
});
