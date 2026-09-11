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
