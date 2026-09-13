import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { enableAutoUnmount, flushPromises, mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import GeometryPanel from "../../../../src-web/views/geometry/GeometryPanel.vue";
import { useGeometryPanel } from "../../../../src-web/views/geometry/useGeometryPanel";
import { useAppStore } from "../../../../src-web/stores/app";
import { useGeometryStore } from "../../../../src-web/stores/geometry";
import {
  estimateVolumeMesh,
  generateDualDomainMesh,
  generateGmshMesh,
  generateMidplaneMesh,
  generateVolumeMesh,
  importSampleBox,
  importGeometryFile,
  removeGeometry,
  repairGeometry,
} from "../../../../src-web/api/geometry";
import { pickOpenGeometryPath } from "../../../../src-web/api/dialog";
import type { GeometrySummary, MeshEstimate, MeshingReport } from "../../../../src-web/types";

function estimateFixture(overrides: Partial<MeshEstimate> = {}): MeshEstimate {
  return {
    engine: "voxel",
    cellCount: 8,
    elementCount: 40,
    overLimit: false,
    basis: "包围盒上限",
    cellLimit: 2_000_000,
    ...overrides,
  };
}

vi.mock("../../../../src-web/api/geometry", () => ({
  estimateVolumeMesh: vi.fn(),
  importGeometryFile: vi.fn(),
  removeGeometry: vi.fn(),
  repairGeometry: vi.fn(),
  generateVolumeMesh: vi.fn(),
  importSampleBox: vi.fn(),
  getRenderMesh: vi.fn(),
  generateGmshMesh: vi.fn(),
  generateDualDomainMesh: vi.fn(),
  generateMidplaneMesh: vi.fn(),
}));
vi.mock("../../../../src-web/api/dialog", () => ({
  pickOpenProjectPath: vi.fn(),
  pickSaveProjectPath: vi.fn(),
  pickOpenJsonPath: vi.fn(),
  pickExportJsonPath: vi.fn(),
  pickOpenGeometryPath: vi.fn(),
}));

function geometryFixture(overrides: Partial<GeometrySummary> = {}): GeometrySummary {
  return {
    geometryId: "geo-1",
    fileName: "demo.stl",
    triangleCount: 12,
    size: [10, 20, 30],
    surfaceArea: 2200,
    signedVolume: 6000,
    suggestedUnit: "mm",
    issues: { degenerate: 0, openEdges: 0, nonManifoldEdges: 0, normalInconsistentEdges: 0 },
    ...overrides,
  };
}

function meshReportFixture(): MeshingReport {
  return {
    engine: "voxel",
    nodeCount: 8,
    elementCount: 12,
    surfaceFaceCount: 6,
    totalVolume: 1000,
    quality: { minEdgeRatio: 0.7, avgEdgeRatio: 0.85, maxEdgeRatio: 0.99, minVolume: 1 },
    aspectMax: 3.4,
    aspectAvg: 1.6,
    thinFeatureHints: [],
  };
}

function findButton(wrapper: ReturnType<typeof mount>, label: string) {
  const found = wrapper.findAll("button").find((candidate) => candidate.text() === label);
  if (found === undefined) {
    throw new Error(`找不到按钮：${label}`);
  }
  return found;
}

// 防抖估算用真实/假计时器混跑：用例结束统一卸载组件（触发 scope 释放清掉挂起计时器），
// 否则漏网的 300ms 计时器会在后续用例里触发额外 IPC 调用（套件并行加载时暴露）。
enableAutoUnmount(afterEach);

describe("GeometryPanel", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.resetAllMocks();
  });

  // 防抖用例使用假计时器：无论断言是否通过都还原，避免污染后续用例
  afterEach(() => {
    vi.useRealTimers();
  });

  it("空状态显示引导文案", () => {
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("尚未导入几何。支持 STL / STEP / IGES。");
  });

  it("导入样例：摘要行、健康文案与按最大边二十分之一的表单初值", async () => {
    vi.mocked(importSampleBox).mockResolvedValue({ summary: geometryFixture(), log: [] });
    const geometry = useGeometryStore();
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });

    await findButton(wrapper, "导入样例").trigger("click");
    await flushPromises();

    expect(importSampleBox).toHaveBeenCalledWith(10);
    expect(geometry.geometries).toHaveLength(1);
    expect(wrapper.text()).toContain("demo.stl");
    expect(wrapper.text()).toContain("12 三角形 · 10.00 × 20.00 × 30.00 mm");
    expect(wrapper.text()).toContain("网格健康");
    expect(wrapper.find("p.text-emerald-400").exists()).toBe(true);
    // 表单初始值：max(10,20,30) / 20 = 1.5 → "1.50"。
    expect(wrapper.find("input").element.getAttribute("value")).toBe("1.50");
    // 未生成网格时报告行给出引导语。
    expect(wrapper.text()).toContain("划分体积网格供求解使用。");
  });

  it("导入样例失败：错误进入全局状态", async () => {
    vi.mocked(importSampleBox).mockRejectedValue(new Error("样例缺失"));
    const app = useAppStore();
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });

    await findButton(wrapper, "导入样例").trigger("click");
    await flushPromises();

    await vi.waitFor(() => expect(app.error?.message).toBe("样例缺失"));
    expect(app.busy).toBeNull();
  });

  it("导入几何：取消不触发 IPC；成功入列；失败进全局错误", async () => {
    vi.mocked(importGeometryFile).mockResolvedValue({
      summary: geometryFixture({ geometryId: "geo-2" }),
      log: [],
    });
    const geometry = useGeometryStore();
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });

    vi.mocked(pickOpenGeometryPath).mockResolvedValue(null);
    await findButton(wrapper, "导入几何").trigger("click");
    await flushPromises();
    expect(importGeometryFile).not.toHaveBeenCalled();

    vi.mocked(pickOpenGeometryPath).mockResolvedValue("/模型/demo.stl");
    await findButton(wrapper, "导入几何").trigger("click");
    await flushPromises();
    expect(importGeometryFile).toHaveBeenCalledWith("/模型/demo.stl");
    expect(geometry.geometries).toHaveLength(1);

    vi.mocked(importGeometryFile).mockRejectedValue(new Error("非二进制 STL"));
    await findButton(wrapper, "导入几何").trigger("click");
    await flushPromises();
    await vi.waitFor(() => expect(useAppStore().error?.message).toBe("非二进制 STL"));
  });

  it("网格健康摘要：四类问题逐项拼接，干净几何显示健康并着绿色", () => {
    const geometry = useGeometryStore();
    geometry.geometries = [
      geometryFixture({
        geometryId: "geo-bad",
        fileName: "bad.stl",
        issues: {
          degenerate: 1,
          openEdges: 2,
          nonManifoldEdges: 3,
          normalInconsistentEdges: 4,
        },
      }),
      geometryFixture({ geometryId: "geo-ok", fileName: "ok.stl" }),
      geometryFixture({
        geometryId: "geo-partial",
        fileName: "partial.stl",
        issues: {
          degenerate: 5,
          openEdges: 0,
          nonManifoldEdges: 0,
          normalInconsistentEdges: 0,
        },
      }),
    ];
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });

    expect(wrapper.text()).toContain("开放边 2，退化三角形 1，非流形边 3，法向不一致 4");
    expect(wrapper.find("p.text-amber-400").exists()).toBe(true);
    expect(wrapper.text()).toContain("网格健康");
    expect(wrapper.text()).toContain("退化三角形 5");
  });

  it("修复报告行：无报告显示占位，翻转法向等非零项全列", async () => {
    const geometry = useGeometryStore();
    geometry.geometries = [
      geometryFixture({ geometryId: "geo-a" }),
      geometryFixture({ geometryId: "geo-b", fileName: "other.stl" }),
    ];
    geometry.repairReports["geo-a"] = {
      mergedVertices: 0,
      removedDegenerate: 0,
      filledHoles: 0,
      filledTriangles: 0,
      flippedFaces: 2,
      selfIntersections: 1,
    };
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("修复：翻转法向 2 · 自交 1");
    // 无修复记录的几何行显示占位文案
    expect(wrapper.text()).toContain("尚未修复。");
  });

  it("修复：不健康几何按钮可用并调用 IPC，健康几何禁用", async () => {
    const geometry = useGeometryStore();
    geometry.geometries = [
      geometryFixture({
        geometryId: "geo-bad",
        fileName: "bad.stl",
        issues: {
          degenerate: 1,
          openEdges: 2,
          nonManifoldEdges: 0,
          normalInconsistentEdges: 0,
        },
      }),
    ];
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });

    vi.mocked(repairGeometry).mockResolvedValue({
      summary: geometryFixture({
        geometryId: "geo-bad",
        fileName: "bad.stl",
        issues: {
          degenerate: 0,
          openEdges: 0,
          nonManifoldEdges: 0,
          normalInconsistentEdges: 0,
        },
      }),
      report: {
        mergedVertices: 2,
        removedDegenerate: 1,
        filledHoles: 1,
        filledTriangles: 3,
        flippedFaces: 0,
        selfIntersections: 0,
      },
    });

    const buttons = wrapper.findAll("button");
    const repairButton = buttons.find((button) => button.text() === "修复")!;
    expect(repairButton.attributes("disabled")).toBeUndefined();

    await repairButton.trigger("click");
    await flushPromises();
    expect(repairGeometry).toHaveBeenCalledWith("geo-bad");

    // 修复后摘要刷新（不健康问题清零）→ 按钮转为禁用；修复报告行渲染计数
    expect(wrapper.text()).toContain("网格健康");
    expect(wrapper.text()).toContain("修复：焊接顶点 2 · 去退化 1 · 填孔 1（+3 面） · 自交 0");
    expect(repairButton.attributes("disabled")).toBeDefined();
  });

  it("生成体积网格（体素）：目标尺寸可编辑并透传；报告行渲染统计", async () => {
    const geometry = useGeometryStore();
    geometry.geometries = [geometryFixture()];
    vi.mocked(generateVolumeMesh).mockResolvedValue(meshReportFixture());
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });

    await wrapper.find("input").setValue("2.5");
    await findButton(wrapper, "生成体积网格").trigger("click");
    await flushPromises();

    expect(generateVolumeMesh).toHaveBeenCalledWith("geo-1", 2.5, undefined);
    expect(geometry.meshReports["geo-1"]).toBeDefined();
    expect(wrapper.text()).toContain(
      "节点 8 · 四面体 12 · 表面 6 · 体积 1000.000 · 质量比 min 0.70 / avg 0.85 / max 0.99 · 纵横比 avg 1.60 / max 3.40",
    );
  });

  it("网格报告带薄壁提示时以警告行渲染", async () => {
    const geometry = useGeometryStore();
    geometry.geometries = [geometryFixture()];
    geometry.meshReports = {
      "geo-1": {
        ...meshReportFixture(),
        thinFeatureHints: ["目标尺寸 5.00 mm 超过最薄特征的 1/2。"],
      },
    };
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });
    await flushPromises();

    const hint = wrapper.findAll("p").find((node) => node.text().includes("最薄特征"));
    expect(hint).toBeDefined();
    expect(hint?.classes()).toContain("text-amber-400");
  });

  it("体素引擎填写边界层数后透传分级加密选项", async () => {
    const geometry = useGeometryStore();
    geometry.geometries = [geometryFixture()];
    vi.mocked(generateVolumeMesh).mockResolvedValue(meshReportFixture());
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });

    await wrapper.find('input[placeholder="边界层数"]').setValue("2");
    await findButton(wrapper, "生成体积网格").trigger("click");
    await flushPromises();

    expect(generateVolumeMesh).toHaveBeenCalledWith("geo-1", 1.5, {
      mode: "boundaryLayers",
      layers: 2,
      ratio: 0.5,
    });
  });

  it("切换 Gmsh 引擎后按引擎分派生成", async () => {
    const geometry = useGeometryStore();
    geometry.geometries = [geometryFixture()];
    vi.mocked(generateGmshMesh).mockResolvedValue(meshReportFixture());
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });

    await wrapper.find("select").setValue("gmsh");
    await findButton(wrapper, "生成体积网格").trigger("click");
    await flushPromises();

    expect(generateGmshMesh).toHaveBeenCalledWith("geo-1", 1.5);
    expect(generateVolumeMesh).not.toHaveBeenCalled();
  });

  it("生成失败：错误进入全局状态", async () => {
    const geometry = useGeometryStore();
    geometry.geometries = [geometryFixture()];
    vi.mocked(generateVolumeMesh).mockRejectedValue(new Error("网格退化"));
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });

    await findButton(wrapper, "生成体积网格").trigger("click");
    await flushPromises();

    await vi.waitFor(() => expect(useAppStore().error?.message).toBe("网格退化"));
  });

  it("双域网格：透传当前方案杆系并渲染报告行", async () => {
    const geometry = useGeometryStore();
    geometry.geometries = [geometryFixture()];
    vi.mocked(generateDualDomainMesh).mockResolvedValue({
      nodeCount: 8,
      triangleCount: 12,
      beamCount: 2,
      couplingCount: 1,
      uncoupledEndpoints: 3,
      unpairedTriangles: 0,
      matchRatio: 1,
      thicknessMin: 1.8,
      thicknessMax: 2.2,
      thicknessAvg: 2.0,
    });
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });

    // 未生成时显示引导语。
    expect(wrapper.text()).toContain("表面厚度配对 + 杆系梁耦合");

    await findButton(wrapper, "双域网格").trigger("click");
    await flushPromises();

    expect(generateDualDomainMesh).toHaveBeenCalledWith("geo-1", []);
    expect(geometry.dualDomainReports["geo-1"]).toBeDefined();
    expect(wrapper.text()).toContain(
      "三角形 12 · 匹配率 100.0% · 厚度 1.80 ~ 2.20（avg 2.00）· 未配对 0 · 梁 2（耦合 1 / 自由 3）",
    );
  });

  it("中面网格：透传杆系并渲染报告行", async () => {
    const geometry = useGeometryStore();
    geometry.geometries = [geometryFixture()];
    vi.mocked(generateMidplaneMesh).mockResolvedValue({
      nodeCount: 8,
      elementCount: 4,
      beamCount: 1,
      couplingCount: 1,
      uncoupledEndpoints: 1,
      unpairedVertices: 0,
      droppedElements: 0,
      thicknessMin: 2,
      thicknessMax: 2,
      thicknessAvg: 2,
    });
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });

    // 未生成时显示引导语。
    expect(wrapper.text()).toContain("顶点配对中面抽取");

    await findButton(wrapper, "中面网格").trigger("click");
    await flushPromises();

    expect(generateMidplaneMesh).toHaveBeenCalledWith("geo-1", []);
    expect(geometry.midplaneReports["geo-1"]).toBeDefined();
    expect(wrapper.text()).toContain(
      "单元 4 · 节点 8 · 厚度 2.00 ~ 2.00（avg 2.00）· 丢弃 0 · 梁 1（耦合 1）",
    );
  });

  it("移除几何：调用 IPC 并从列表消失；失败进全局错误", async () => {
    const geometry = useGeometryStore();
    geometry.geometries = [geometryFixture()];
    vi.mocked(removeGeometry).mockResolvedValue(undefined);
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });

    await findButton(wrapper, "移除").trigger("click");
    await flushPromises();

    expect(removeGeometry).toHaveBeenCalledWith("geo-1");
    expect(geometry.geometries).toHaveLength(0);
    expect(wrapper.text()).toContain("尚未导入几何。支持 STL / STEP / IGES。");

    geometry.geometries = [geometryFixture()];
    await nextTick();
    vi.mocked(removeGeometry).mockRejectedValue(new Error("移除失败"));
    await findButton(wrapper, "移除").trigger("click");
    await flushPromises();
    await vi.waitFor(() => expect(useAppStore().error?.message).toBe("移除失败"));
  });

  it("忙碌中：导入 / 移除 / 生成按钮禁用", async () => {
    const geometry = useGeometryStore();
    geometry.geometries = [geometryFixture()];
    const app = useAppStore();
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });

    app.beginBusy("正在导入几何…");
    await nextTick();
    expect(findButton(wrapper, "导入几何").attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "移除").attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "生成体积网格").attributes("disabled")).toBeDefined();
  });

  it("目标尺寸变化后防抖估算单元规模，超限标红", async () => {
    vi.useFakeTimers();
    const geometry = useGeometryStore();
    geometry.geometries = [geometryFixture()];
    vi.mocked(estimateVolumeMesh).mockResolvedValue(estimateFixture());
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });

    // 挂载即排入一次估算（导入后可立刻看到量级）→ 防抖窗口内不请求 IPC。
    expect(estimateVolumeMesh).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(300);
    expect(estimateVolumeMesh).toHaveBeenCalledWith("geo-1", 1.5, undefined, "voxel");
    expect(wrapper.text()).toContain("约 40 单元（包围盒上限）");

    // 连续编辑只保留最后一次：防抖窗口内再改，计时器被清掉，只发一次末次尺寸的请求。
    // （两次编辑之间不推进时钟——否则测的是「恰好谁先到」，而不是防抖语义。）
    vi.mocked(estimateVolumeMesh).mockClear();
    await wrapper.find("input").setValue("2.5");
    await nextTick();
    await wrapper.find("input").setValue("3.5");
    await vi.advanceTimersByTimeAsync(300);
    expect(estimateVolumeMesh).toHaveBeenCalledTimes(1);
    expect(estimateVolumeMesh).toHaveBeenCalledWith("geo-1", 3.5, undefined, "voxel");

    // 超限估算：文案带告警且着警示色。
    vi.mocked(estimateVolumeMesh).mockResolvedValue(
      estimateFixture({ overLimit: true, elementCount: 1_000_000, cellLimit: 2_000_000 }),
    );
    await wrapper.find("input").setValue("0.01");
    await vi.advanceTimersByTimeAsync(300);
    await nextTick();
    expect(wrapper.text()).toContain("超过上限 2,000,000 体素，请调大目标尺寸。");
    const estimateLine = wrapper.findAll("p").find((node) => node.text().includes("超过上限"));
    expect(estimateLine?.classes()).toContain("text-rose-400");

    wrapper.unmount();
    vi.useRealTimers();
  });

  it("切换引擎与边界层数按对应口径估算", async () => {
    vi.useFakeTimers();
    const geometry = useGeometryStore();
    geometry.geometries = [geometryFixture()];
    vi.mocked(estimateVolumeMesh).mockResolvedValue(
      estimateFixture({ engine: "gmsh", cellCount: 0, basis: "体积粗估" }),
    );
    const wrapper = mount(GeometryPanel, { global: { plugins: [pinia] } });
    await nextTick();
    await vi.advanceTimersByTimeAsync(300);

    vi.mocked(estimateVolumeMesh).mockClear();
    await wrapper.find("select").setValue("gmsh");
    await vi.advanceTimersByTimeAsync(300);
    expect(estimateVolumeMesh).toHaveBeenCalledWith("geo-1", 1.5, undefined, "gmsh");
    await nextTick();
    expect(wrapper.text()).toContain("约 40 单元（体积粗估）");

    // 体素 + 边界层数：加密选项透传。
    vi.mocked(estimateVolumeMesh).mockClear();
    await wrapper.find("select").setValue("voxel");
    await vi.advanceTimersByTimeAsync(300);
    await wrapper.find('input[placeholder="边界层数"]').setValue("2");
    await vi.advanceTimersByTimeAsync(300);
    expect(estimateVolumeMesh).toHaveBeenLastCalledWith(
      "geo-1",
      1.5,
      {
        mode: "boundaryLayers",
        layers: 2,
        ratio: 0.5,
      },
      "voxel",
    );

    // 未经防抖的旧计时器在卸载时清理：卸载后再推进时间不再触发请求。
    vi.mocked(estimateVolumeMesh).mockClear();
    await wrapper.find("input").setValue("4");
    wrapper.unmount();
    await vi.advanceTimersByTimeAsync(1000);
    expect(estimateVolumeMesh).not.toHaveBeenCalled();
    vi.useRealTimers();
  });

  it("估算文案：无估算留空，未超限只给量级与依据", () => {
    const panel = useGeometryPanel();
    expect(panel.estimateText(undefined)).toBe("");
    expect(panel.estimateOverLimit(undefined)).toBe(false);
    expect(panel.estimateOverLimit(estimateFixture())).toBe(false);
    expect(panel.estimateText(estimateFixture({ elementCount: 40000 }))).toBe(
      "约 40,000 单元（包围盒上限）",
    );
  });

  it("行表单惰性建表：渲染前建表、重渲染复用、watch 跳过已有项", async () => {
    const geometry = useGeometryStore();
    // 直接调用 composable：挂载前 watch(immediate) 遇到空列表不建表。
    const panel = useGeometryPanel();
    geometry.geometries = [geometryFixture()]; // 同步赋值，watcher 尚未 flush
    // rows 首次求值时 watch 还没跑 → meshForm 现场建表（建议尺寸）。
    expect(panel.rows.value[0]?.form.size).toBe("1.50");
    expect(panel.rows.value[0]?.form.engine).toBe("voxel");
    // 列表重建（新数组引用）→ rows 重算 → meshForm 复用既有表单（用户编辑不回填）。
    panel.rows.value[0]!.form.size = "9";
    geometry.geometries = [geometryFixture()];
    expect(panel.rows.value[0]?.form.size).toBe("9");
    // watcher flush：条目已存在，跳过初始化（false 分支）。
    await nextTick();
    expect(panel.rows.value[0]?.form.size).toBe("9");
  });
});
