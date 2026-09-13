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

/** 视口拾取放置状态：模具网络面板开启，下一次视口点击回填坐标。 */
interface PlacementState {
  /** 是否等待视口点击。 */
  active: boolean;
  /** 拾取后是否保持活动（连续放置多个浇口）。 */
  continuous: boolean;
  /** 最近一次吸附后的拾取点（模型单位）；null = 尚未拾取。 */
  point: [number, number, number] | null;
  /** 本次放置模式的累计拾取次数（驱动面板提示刷新）。 */
  picks: number;
}

const IDLE_PLACEMENT: PlacementState = {
  active: false,
  continuous: false,
  point: null,
  picks: 0,
};

export const useViewportStore = defineStore("viewport", {
  state: () => ({
    layers: { ...DEFAULT_VISIBLE } as LayerState,
    /** 视口布局：单视口 / 四分格（多视口联动）。 */
    layout: "single" as ViewportLayout,
    /** 视口是否已载入网格：拾取放置的前置条件，跨面板共享。 */
    meshLoaded: false,
    /** 视口拾取放置状态。 */
    placement: { ...IDLE_PLACEMENT } as PlacementState,
  }),
  actions: {
    /** 视口网格载入 / 失效：失效时一并退出放置模式。 */
    setMeshLoaded(loaded: boolean): void {
      this.meshLoaded = loaded;
      if (!loaded) {
        this.placement = { ...this.placement, active: false };
      }
    },
    /** 进入拾取放置模式（连续模式拾取后继续等待下一次点击）。 */
    beginPlacement(continuous = false): void {
      this.placement = { active: true, continuous, point: null, picks: 0 };
    },
    /** 退出放置模式（保留最近拾取点）。 */
    cancelPlacement(): void {
      this.placement = { ...this.placement, active: false };
    },
    /** 视口拾取回调：写入吸附点，单次模式拾取后自动退出。 */
    recordPick(point: [number, number, number]): void {
      this.placement = {
        ...this.placement,
        active: this.placement.continuous,
        point,
        picks: this.placement.picks + 1,
      };
    },
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
