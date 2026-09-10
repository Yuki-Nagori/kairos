import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import LayersPanel from "../../../../src-web/views/layers/LayersPanel.vue";
import { useGeometryStore } from "../../../../src-web/stores/geometry";
import { useProjectStore } from "../../../../src-web/stores/project";
import { useViewportStore } from "../../../../src-web/stores/viewport";
import type { MeshingReport, Project, Study } from "../../../../src-web/types";

function studyFixture(overrides: Partial<Study> = {}): Study {
  return {
    id: "study-1",
    name: "方案 A",
    createdMs: 1,
    runnerElements: [
      { id: "g1", kind: "gate", diameterMm: 2, start: [0, 0, 0], end: [1, 0, 0] },
      { id: "r1", kind: "runner", diameterMm: 6, start: [1, 0, 0], end: [5, 0, 0] },
    ],
    coolingChannels: [
      { id: "c1", diameterMm: 8, start: [0, 5, 0], end: [5, 5, 0], inletTempC: 25 },
    ],
    process: null,
    materialId: null,
    ...overrides,
  };
}

function projectFixture(study: Study): Project {
  return {
    schemaVersion: 1,
    id: "p-1",
    name: "演示项目",
    createdMs: 1,
    updatedMs: 1,
    studies: [study],
  };
}

function reportFixture(): MeshingReport {
  return {
    engine: "voxel",
    nodeCount: 100,
    elementCount: 5000,
    surfaceFaceCount: 120,
    totalVolume: 1000,
    quality: {
      minEdgeRatio: 0.4,
      avgEdgeRatio: 0.8,
      maxEdgeRatio: 1.2,
      minVolume: 0.01,
    },
  };
}

/** 找到指定标签的图层行按钮。 */
function row(wrapper: ReturnType<typeof mount>, label: string) {
  return wrapper.findAll("button").find((button) => button.text().includes(label))!;
}

describe("LayersPanel", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("无研究时四个图层都不可用（◎ 熄灭态）", () => {
    const wrapper = mount(LayersPanel, { global: { plugins: [pinia] } });
    expect(wrapper.findAll("button").length).toBe(4);
    expect(wrapper.text()).toContain("制品网格");
    expect(wrapper.text()).not.toContain("5");
  });

  it("研究带浇口/流道/水路时行可用并显示数量", () => {
    const project = useProjectStore();
    project.project = projectFixture(studyFixture());
    project.activeStudyId = "study-1";
    const geometry = useGeometryStore();
    geometry.meshReports["geo-1"] = reportFixture();

    const wrapper = mount(LayersPanel, { global: { plugins: [pinia] } });
    expect(row(wrapper, "浇口").text()).toContain("1");
    expect(row(wrapper, "流道").text()).toContain("1");
    expect(row(wrapper, "冷却水路").text()).toContain("1");
    expect(row(wrapper, "制品网格").text()).toContain("5000");
    expect(row(wrapper, "制品网格").text()).toContain("◉");
  });

  it("点击可用行切换 store 可见性，无数据行点击无效", async () => {
    const project = useProjectStore();
    project.project = projectFixture(studyFixture());
    project.activeStudyId = "study-1";
    const viewport = useViewportStore();
    const wrapper = mount(LayersPanel, { global: { plugins: [pinia] } });

    await row(wrapper, "浇口").trigger("click");
    expect(viewport.layers.gates).toBe(false);
    expect(row(wrapper, "浇口").text()).toContain("◎");

    await row(wrapper, "浇口").trigger("click");
    expect(viewport.layers.gates).toBe(true);

    // 无研究时冷却行不可用，点击不改变状态
    project.project = null;
    await wrapper.vm.$nextTick();
    await row(wrapper, "冷却水路").trigger("click");
    expect(viewport.layers.cooling).toBe(true);
  });
});
