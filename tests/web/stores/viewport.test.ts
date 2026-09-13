import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import { useViewportStore } from "../../../src-web/stores/viewport";

describe("viewport store", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("默认全部图层可见", () => {
    const viewport = useViewportStore();
    expect(viewport.layers).toEqual({ mesh: true, gates: true, runners: true, cooling: true });
  });

  it("切换单个图层不影响其他图层", () => {
    const viewport = useViewportStore();
    viewport.setLayerVisible("gates", false);
    viewport.setLayerVisible("mesh", false);
    expect(viewport.layers.gates).toBe(false);
    expect(viewport.layers.mesh).toBe(false);
    expect(viewport.layers.runners).toBe(true);
    expect(viewport.layers.cooling).toBe(true);
  });

  it("重复切换同一图层为幂等写入", () => {
    const viewport = useViewportStore();
    viewport.setLayerVisible("cooling", false);
    viewport.setLayerVisible("cooling", false);
    expect(viewport.layers.cooling).toBe(false);
  });

  it("resetLayers 恢复默认全开", () => {
    const viewport = useViewportStore();
    viewport.setLayerVisible("gates", false);
    viewport.resetLayers();
    expect(viewport.layers).toEqual({ mesh: true, gates: true, runners: true, cooling: true });
  });

  describe("layout（多视口联动）", () => {
    it("默认单视口，setLayout 切换四分格", () => {
      const viewport = useViewportStore();
      expect(viewport.layout).toBe("single");

      viewport.setLayout("quad");
      expect(viewport.layout).toBe("quad");

      viewport.setLayout("single");
      expect(viewport.layout).toBe("single");
    });
  });
});

describe("视口拾取放置", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("载入标志置位与复位；复位时退出放置模式", () => {
    const viewport = useViewportStore();
    expect(viewport.meshLoaded).toBe(false);
    expect(viewport.placement).toEqual({ active: false, continuous: false, point: null, picks: 0 });

    viewport.setMeshLoaded(true);
    expect(viewport.meshLoaded).toBe(true);

    viewport.beginPlacement();
    expect(viewport.placement.active).toBe(true);
    viewport.setMeshLoaded(false);
    expect(viewport.meshLoaded).toBe(false);
    expect(viewport.placement.active).toBe(false);
  });

  it("单次模式：拾取后退出并记录点与计数", () => {
    const viewport = useViewportStore();
    viewport.beginPlacement();
    viewport.recordPick([1, 2, 3]);
    expect(viewport.placement).toEqual({
      active: false,
      continuous: false,
      point: [1, 2, 3],
      picks: 1,
    });
    viewport.recordPick([4, 5, 6]);
    expect(viewport.placement.point).toEqual([4, 5, 6]);
    expect(viewport.placement.picks).toBe(2);
  });

  it("连续模式：拾取后保持活动；beginPlacement 重置计数", () => {
    const viewport = useViewportStore();
    viewport.beginPlacement(true);
    viewport.recordPick([0, 0, 1]);
    expect(viewport.placement.active).toBe(true);
    expect(viewport.placement.picks).toBe(1);

    viewport.cancelPlacement();
    expect(viewport.placement.active).toBe(false);
    expect(viewport.placement.point).toEqual([0, 0, 1]);

    viewport.beginPlacement();
    expect(viewport.placement).toEqual({ active: true, continuous: false, point: null, picks: 0 });
  });
});
