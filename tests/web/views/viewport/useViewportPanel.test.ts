/**
 * 视口面板 composable 全行为测试（mock 渲染后端）。
 * 该文件曾长期被覆盖率口径排除（happy-dom 无 WebGL2），期间 T49 重构丢失
 * slot.renderMesh 赋值导致云图 / 剖切 / 拾取静默失效而无人察觉——本文件
 * 即为该教训的收编：后端 mock 后逐行为断言，100% 覆盖含分支。
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { defineComponent, h, nextTick, reactive, type Ref } from "vue";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import { useViewportPanel } from "../../../../src-web/views/viewport/useViewportPanel";
import { useViewportStore } from "../../../../src-web/stores/viewport";
import { useResultsStore } from "../../../../src-web/stores/results";
import { useGeometryStore } from "../../../../src-web/stores/geometry";
import { useProjectStore } from "../../../../src-web/stores/project";
import { registerSnapshot } from "../../../../src-web/render/snapshot";
import type { Mock } from "vitest";
import type { ViewportBackend } from "../../../../src-web/render/backend";
import type { ScalarField } from "../../../../src-web/types";

const { createMock, backends, capabilityMock, baseCreate } = vi.hoisted(() => {
  type Backend = Record<keyof ViewportBackend, ReturnType<typeof vi.fn>> & {
    callbacks: { onFps?: (fps: number) => void; onView?: (state: unknown) => void };
  };
  const backends: Backend[] = [];
  const baseCreate = async (
    _canvas: HTMLCanvasElement,
    onFps?: (fps: number) => void,
    onView?: (state: unknown) => void,
  ): Promise<{ backend: ViewportBackend; kind: "webgpu" } | null> => {
    const backend = {
      uploadMesh: vi.fn(),
      setFaceValues: vi.fn(),
      setFieldRange: vi.fn(),
      setClipPlane: vi.fn(),
      uploadOverlay: vi.fn(),
      setMeshVisible: vi.fn(),
      setOverlayVisible: vi.fn(),
      getCamera: vi.fn(() => ({
        eye: [0, 0, 5],
        target: [0, 0, 0],
        fovY: Math.PI / 4,
        aspect: 1,
        width: 100,
        height: 100,
      })),
      getOrbit: vi.fn(() => ({ x: 1, y: 2, z: 3, yaw: 0.6, pitch: 0.4, distance: 3 })),
      setOrbit: vi.fn(),
      getMeshBounds: vi.fn(() => ({ min: [0, 0, 0], max: [1, 1, 1] })),
      resetView: vi.fn(),
      zoomBy: vi.fn(),
      fitView: vi.fn(),
      dispose: vi.fn(),
      callbacks: { onFps, onView },
    } as Backend;
    backends.push(backend);
    return { backend: backend as unknown as ViewportBackend, kind: "webgpu" as const };
  };
  const createMock = vi.fn(baseCreate);
  const capabilityMock = vi.fn(async () => ({ backend: "webgpu", note: "WebGPU 后端" }));
  return { createMock, backends, capabilityMock, baseCreate };
});

vi.mock("../../../../src-web/render/backend", () => ({
  createViewportRenderer: createMock,
}));
vi.mock("../../../../src-web/render/capability", () => ({
  detectRenderCapabilityInBrowser: capabilityMock,
}));
vi.mock("../../../../src-web/render/snapshot", () => ({ registerSnapshot: vi.fn() }));
const { pickCellMock, rayMock } = vi.hoisted(() => ({
  pickCellMock: vi.fn(),
  rayMock: vi.fn(() => ({ origin: [0, 0, 5], dir: [0, 0, -1] })),
}));
vi.mock("../../../../src-web/render/picking", () => ({
  pickCell: pickCellMock,
  rayFromPointer: rayMock,
}));
vi.mock("../../../../src-web/api/geometry", () => ({ getRenderMesh: vi.fn() }));
vi.mock("../../../../src-web/api/results", () => ({
  listResultTimes: vi.fn(),
  loadResultField: vi.fn(),
  deriveField: vi.fn(),
  deriveDifference: vi.fn(),
}));
vi.mock("../../../../src-web/api/project", () => ({
  createProject: vi.fn(),
  listRecentProjects: vi.fn(),
  loadProjectFile: vi.fn(),
  saveProjectFile: vi.fn(),
}));
vi.mock("../../../../src-web/api/materials", () => ({
  listBuiltinMaterials: vi.fn(async () => []),
  listCustomMaterials: vi.fn(async () => []),
  importCustomMaterials: vi.fn(),
  upsertCustomMaterial: vi.fn(),
  deleteCustomMaterial: vi.fn(),
  exportMaterialsToFile: vi.fn(),
}));

function canvas(): HTMLCanvasElement {
  return document.createElement("canvas");
}

function makeField(values: number[]): ScalarField {
  return {
    field: "p",
    timeDir: "0.100",
    timeS: 0.1,
    values,
    isMagnitude: false,
    complete: true,
  };
}

let pinia: Pinia;

beforeEach(() => {
  pinia = createPinia();
  setActivePinia(pinia);
  backends.length = 0;
  createMock.mockReset();
  createMock.mockImplementation(baseCreate);
  pickCellMock.mockReset();
  rayMock.mockReset();
  rayMock.mockImplementation(() => ({ origin: [0, 0, 5], dir: [0, 0, -1] }));
  capabilityMock.mockResolvedValue({ backend: "webgpu", note: "WebGPU 后端" });
  vi.mocked(registerSnapshot).mockClear();
});

afterEach(() => {
  vi.useRealTimers();
});

const Harness = defineComponent({
  setup() {
    const panel = useViewportPanel();
    return { panel };
  },
  render() {
    return h("div");
  },
});

type Unwrapped<T> = { [K in keyof T]: T[K] extends Ref<infer V> ? V : T[K] };

interface Mounted {
  panel: Unwrapped<ReturnType<typeof useViewportPanel>> & { unmount: () => void };
  unmount: () => void;
}

function mountPanel(): Mounted {
  const wrapper = mount(Harness, { global: { plugins: [pinia] } });
  const raw = (wrapper.vm as unknown as { panel: ReturnType<typeof useViewportPanel> }).panel;
  // reactive 包一层：refs 在属性访问时自动解包（可读可写）
  const panel = reactive(raw) as unknown as Unwrapped<ReturnType<typeof useViewportPanel>> & {
    unmount: () => void;
  };
  Object.assign(panel, { unmount: () => wrapper.unmount() });
  return { panel, unmount: () => wrapper.unmount() };
}

/** 登记一个活跃研究（叠加层上传路径需要）。 */
function primeStudy(): void {
  const project = useProjectStore();
  project.project = {
    schemaVersion: 4,
    id: "p-1",
    name: "演示项目",
    createdMs: 1,
    updatedMs: 1,
    studies: [
      {
        id: "s-1",
        name: "填充分析",
        createdMs: 1,
        runnerElements: [],
        coolingChannels: [],
        process: null,
        materialId: null,
      },
    ],
  };
  project.activeStudyId = "s-1";
}

/** 主流程夹具：主视口画布就绪 + 几何就绪 + 网格已载入。返回面板与画布引用。 */
async function mountLoaded(): Promise<{
  panel: Unwrapped<ReturnType<typeof useViewportPanel>> & { unmount: () => void };
  el: HTMLCanvasElement;
}> {
  const geometry = useGeometryStore();
  const { getRenderMesh } = (await import("../../../../src-web/api/geometry")) as unknown as {
    getRenderMesh: Mock;
  };
  geometry.geometries = [
    {
      geometryId: "g-1",
      fileName: "demo.stl",
      triangleCount: 2,
      size: [1, 1, 0],
      surfaceArea: 1,
      signedVolume: 0,
      suggestedUnit: "mm",
      issues: { degenerate: 0, openEdges: 0, nonManifoldEdges: 0, normalInconsistentEdges: 0 },
    },
  ];
  vi.mocked(getRenderMesh).mockResolvedValue({
    positions: [0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 1, 0],
    indices: [0, 1, 2, 0, 2, 3],
    faceCells: [0, 1],
  });

  const { panel } = mountPanel();
  const el = canvas();
  panel.attachCanvas(0, el);
  await flushPromises();
  await panel.loadMesh();
  await flushPromises();
  return { panel, el };
}

describe("useViewportPanel：渲染器生命周期", () => {
  it("画布就绪即建渲染器且幂等；载入网格后上传网格、图层与快照", async () => {
    const geometry = useGeometryStore();
    const { getRenderMesh } = (await import("../../../../src-web/api/geometry")) as unknown as {
      getRenderMesh: ReturnType<typeof vi.fn>;
    };
    geometry.geometries = [
      {
        geometryId: "g-1",
        fileName: "demo.stl",
        triangleCount: 2,
        size: [1, 1, 0],
        surfaceArea: 1,
        signedVolume: 0,
        suggestedUnit: "mm",
        issues: { degenerate: 0, openEdges: 0, nonManifoldEdges: 0, normalInconsistentEdges: 0 },
      },
    ];
    vi.mocked(getRenderMesh).mockResolvedValue({
      positions: [0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 1, 0],
      indices: [0, 1, 2, 0, 2, 3],
      faceCells: [0, 1],
    });

    primeStudy();
    const { panel } = mountPanel();
    const el = canvas();
    panel.attachCanvas(0, el);
    await flushPromises();
    // 幂等：重复 attach 不重复建渲染器
    panel.attachCanvas(0, el);
    await flushPromises();
    expect(createMock).toHaveBeenCalledTimes(1);
    expect(panel.emptyError).toBe(false);

    await panel.loadMesh();
    await flushPromises();

    const backend = backends[0]!;
    expect(backend.uploadMesh).toHaveBeenCalledTimes(1);
    // 浇注系统叠加层（研究为空 → buildOverlayLayers 空层仍上传三个 id）
    expect(backend.uploadOverlay).toHaveBeenCalledTimes(3);
    // 图层可见性意图同步
    expect(backend.setMeshVisible).toHaveBeenCalledWith(true);
    expect(backend.setOverlayVisible).toHaveBeenCalledWith("gates", true);
    // 载入成功注册快照，标题与就绪态
    expect(registerSnapshot).toHaveBeenCalledWith("viewport", el);
    expect(panel.meshReady).toBe(true);
    expect(panel.title).toBe("demo.stl · 填充分析");
    // 控制条状态（单视口布局）
    expect(panel.loadDisabled).toBe(false);
    expect(panel.slotIds).toEqual([0]);
  });

  it("渲染器创建失败：主视口给出环境不支持提示，不上传网格", async () => {
    createMock.mockImplementation(async () => null);
    const geometry = useGeometryStore();
    const { getRenderMesh } = (await import("../../../../src-web/api/geometry")) as unknown as {
      getRenderMesh: ReturnType<typeof vi.fn>;
    };
    geometry.geometries = [
      {
        geometryId: "g-1",
        fileName: "demo.stl",
        triangleCount: 2,
        size: [1, 1, 0],
        surfaceArea: 1,
        signedVolume: 0,
        suggestedUnit: "mm",
        issues: { degenerate: 0, openEdges: 0, nonManifoldEdges: 0, normalInconsistentEdges: 0 },
      },
    ];
    vi.mocked(getRenderMesh).mockResolvedValue({
      positions: [0, 0, 0],
      indices: [0, 1, 2],
      faceCells: [0],
    });

    const { panel } = mountPanel();
    panel.attachCanvas(0, canvas());
    await flushPromises();
    await panel.loadMesh();
    await flushPromises();

    expect(panel.emptyError).toBe(true);
    expect(panel.emptyText).toContain("当前环境不支持");
    expect(panel.meshReady).toBe(false);
  });

  it("onMounted 能力探测：webgl2 回退说明追加进空态文案", async () => {
    capabilityMock.mockResolvedValue({
      backend: "webgl2",
      note: "WebGPU 不可用，已回退 WebGL2（当前 WebView 尚未支持 WebGPU）。",
    });
    const first = mountPanel();
    await flushPromises();
    expect(first.panel.emptyText).toContain("WebGPU 不可用，已回退 WebGL2");
    first.unmount();
  });
});

describe("useViewportPanel：云图 / 剖切 / 图层（renderMesh 回归锁定）", () => {
  it("场加载后按 faceCells 映射每面值并设置值域（T49 丢失赋值的回归）", async () => {
    const { panel } = await mountLoaded();
    const results = useResultsStore();
    const backend = backends[0]!;

    results.loadedField = makeField([10, 20, 30, 40]);
    await nextTick();
    await flushPromises();

    // faceCells [0,1] → 每面值 [10, 20]
    expect(backend.setFaceValues).toHaveBeenCalledWith(new Float32Array([10, 20]));
    expect(backend.setFieldRange).toHaveBeenCalledWith(10, 40);
    // 图例：max/mid/min（中值 quickselect 取位 2 → 30）
    expect(panel.legendValues).toEqual([40, 30, 10]);
    expect(panel.legendVisible).toBe(true);
    // 无活跃研究 → 标题走「未选择方案」回退
    expect(panel.title).toBe("demo.stl · 未选择方案");

    // 场值比面数短 → 越界面回退 0（applyField 的 ?? 0 分支）
    results.loadedField = makeField([10]);
    await nextTick();
    expect(backend.setFaceValues).toHaveBeenLastCalledWith(new Float32Array([10, 0]));

    // 几何列表清空后（载入态由槽位决定）→ 文件名走「—」回退
    const geometryStore = useGeometryStore();
    geometryStore.geometries = [];
    await nextTick();
    expect(panel.title).toBe("— · 未选择方案");
    geometryStore.geometries = [
      {
        geometryId: "g-1",
        fileName: "demo.stl",
        triangleCount: 2,
        size: [1, 1, 0],
        surfaceArea: 1,
        signedVolume: 0,
        suggestedUnit: "mm",
        issues: { degenerate: 0, openEdges: 0, nonManifoldEdges: 0, normalInconsistentEdges: 0 },
      },
    ];
    await nextTick();

    // 空场 → 图例清空且不再映射
    backend.setFaceValues.mockClear();
    results.loadedField = makeField([]);
    await nextTick();
    results.loadedField = null;
    await nextTick();
    expect(panel.legendValues).toBeNull();
    expect(backend.setFaceValues).not.toHaveBeenCalled();
    panel.unmount();
  });

  it("剖切：toggle 与轴向 / 位置 / 反向变化都按包围盒换算并下发", async () => {
    const { panel } = await mountLoaded();
    const backend = backends[0]!;

    panel.toggleClip();
    expect(backend.setClipPlane).toHaveBeenLastCalledWith(true, [0, 1, 0], 0.5);

    panel.clipPosition = 0.25;
    await nextTick();
    expect(backend.setClipPlane).toHaveBeenLastCalledWith(true, [0, 1, 0], 0.25);

    panel.clipAxis = "x";
    await nextTick();
    expect(backend.setClipPlane).toHaveBeenLastCalledWith(true, [1, 0, 0], 0.25);

    panel.clipInvert = true;
    await nextTick();
    expect(backend.setClipPlane).toHaveBeenLastCalledWith(true, [-1, 0, 0], -0.25);

    panel.toggleClip();
    expect(backend.setClipPlane).toHaveBeenLastCalledWith(false, [-1, 0, 0], -0.25);

    // 包围盒缺失（getMeshBounds null）→ 跳过该实例不下发
    backend.getMeshBounds.mockReturnValueOnce(null);
    panel.clipPosition = 0.75;
    await nextTick();
    expect(backend.getMeshBounds).toHaveBeenCalled();
    panel.unmount();
  });

  it("图层可见性意图变化同步到全部实例", async () => {
    const { panel } = await mountLoaded();
    const viewport = useViewportStore();
    const backend = backends[0]!;

    viewport.setLayerVisible("gates", false);
    await nextTick();
    expect(backend.setOverlayVisible).toHaveBeenCalledWith("gates", false);
    expect(backend.setOverlayVisible).toHaveBeenCalledWith("runners", true);
    panel.unmount();
  });
});

describe("useViewportPanel：多视口联动与相机", () => {
  it("四分格补建实例并共享网格；相机交互联动其余实例", async () => {
    const { panel } = await mountLoaded();
    const viewport = useViewportStore();
    const backend0 = backends[0]!;

    viewport.setLayout("quad");
    await nextTick();
    for (const id of [1, 2, 3]) {
      panel.attachCanvas(id, canvas());
    }
    await flushPromises();

    // 主视口 1 次 + 四分格新增 3 次（并发去重后不重复创建）
    expect(createMock).toHaveBeenCalledTimes(4);
    for (const id of [1, 2, 3]) {
      const backend = backends[id]!;
      expect(backend.uploadMesh).toHaveBeenCalledTimes(1);
    }
    expect(panel.slotIds).toEqual([0, 1, 2, 3]);

    // 主视口交互（onView 回调）→ 注视点读数更新 + 其余实例 setOrbit
    const state = { x: 1, y: 2, z: 3, yaw: 0.6, pitch: 0.4, distance: 3 };
    backend0.callbacks.onView?.(state);
    expect(panel.centerText).toBe("X 1.0 · Y 2.0 · Z 3.0");
    expect(backends[1]!.setOrbit).toHaveBeenCalledWith(state);
    // 每个新增实例的交互回调同样触发联动
    for (const id of [1, 2, 3]) {
      backends[id]!.callbacks.onView?.(state);
    }
    expect(backends[2]!.setOrbit).toHaveBeenCalled();
    expect(backends[3]!.setOrbit).toHaveBeenCalled();

    // resetView / zoomBy / fitView 广播
    panel.resetView();
    expect(backend0.resetView).toHaveBeenCalled();
    expect(backends[1]!.setOrbit).toHaveBeenCalled();
    panel.zoomBy(1.1);
    expect(backend0.zoomBy).toHaveBeenCalledWith(1.1);
    panel.fitView();
    expect(backend0.fitView).toHaveBeenCalled();
    panel.unmount();
  });

  it("渲染器未就绪时 zoomBy 与读数保持缺省（orbit undefined 分支）", async () => {
    const { panel } = mountPanel();
    panel.zoomBy(1.1);
    expect(panel.centerText).toBe("X 0.0 · Y 0.0 · Z 0.0");
    panel.unmount();
  });

  it("主视口 FPS 回调写读数", async () => {
    const { panel } = mountPanel();
    panel.attachCanvas(0, canvas());
    await flushPromises();
    backends[0]!.callbacks.onFps?.(60);
    expect(panel.fpsText).toBe("FPS: 60");
    panel.unmount();
  });
});

describe("useViewportPanel：空间拾取", () => {
  it("pointerdown/up 位移小于阈值且命中单元 → 加入探针", async () => {
    const { panel, el } = await mountLoaded();
    const results = useResultsStore();
    pickCellMock.mockReturnValueOnce({ cell: 3, value: 1 });

    const down = new Event("pointerdown");
    Object.assign(down, { clientX: 10, clientY: 10 });
    el.dispatchEvent(down);
    const up = new Event("pointerup");
    Object.assign(up, { clientX: 12, clientY: 12 });
    el.dispatchEvent(up);

    expect(rayMock).toHaveBeenCalled();
    expect(pickCellMock).toHaveBeenCalled();
    expect(results.probes.map((probe) => probe.nodeIndex)).toEqual([3]);
    expect(panel.probeHits).toBe(1);
    panel.unmount();
  });

  it("位移超过阈值（拖拽）与未命中都不加入探针", async () => {
    const { panel, el } = await mountLoaded();
    const results = useResultsStore();

    // 未按下直接抬起（start === null 分支）
    const stray = new Event("pointerup");
    Object.assign(stray, { clientX: 1, clientY: 1 });
    el.dispatchEvent(stray);
    expect(pickCellMock).not.toHaveBeenCalled();

    // 拖拽：位移 > 4px
    const down = new Event("pointerdown");
    Object.assign(down, { clientX: 0, clientY: 0 });
    el.dispatchEvent(down);
    const drag = new Event("pointerup");
    Object.assign(drag, { clientX: 40, clientY: 0 });
    el.dispatchEvent(drag);
    expect(pickCellMock).not.toHaveBeenCalled();

    // 点击但未命中
    pickCellMock.mockReturnValueOnce(null);
    const down2 = new Event("pointerdown");
    Object.assign(down2, { clientX: 0, clientY: 0 });
    el.dispatchEvent(down2);
    const up2 = new Event("pointerup");
    Object.assign(up2, { clientX: 1, clientY: 1 });
    el.dispatchEvent(up2);
    expect(results.probes).toHaveLength(0);
    expect(panel.probeHits).toBe(0);
    panel.unmount();
  });
});

describe("useViewportPanel：时间步动画", () => {
  it("无目录或空时间步时播放不可用；togglePlay 空转", async () => {
    const { panel } = mountPanel();
    expect(panel.playDisabled).toBe(true);
    panel.togglePlay();
    expect(panel.playLabel).toBe("播放动画");
    panel.unmount();
  });

  it("播放经动画节拍器逐帧加载场并应用云图；再 toggle 停止", async () => {
    vi.useFakeTimers();
    const { panel } = await mountLoaded();
    const results = useResultsStore();
    const { loadResultField } = (await import("../../../../src-web/api/results")) as unknown as {
      loadResultField: Mock;
    };
    results.resultCatalog = {
      caseDir: "/case/run",
      times: [
        { dirName: "1", timeS: 1, fields: ["p"] },
        { dirName: "2", timeS: 2, fields: ["p"] },
      ],
    };
    results.loadedField = null; // 无已加载场 → 动画场名走 "T" 回退
    vi.mocked(loadResultField).mockResolvedValue(makeField([5, 6, 7, 8]));
    await nextTick();
    const backend = backends[0]!;
    backend.setFaceValues.mockClear();

    expect(panel.playDisabled).toBe(false);
    panel.togglePlay();
    expect(panel.playLabel).toBe("停止动画");

    await vi.advanceTimersByTimeAsync(400);
    expect(loadResultField).toHaveBeenCalledWith("/case/run", "1", "T", "primary");
    await Promise.resolve();
    await Promise.resolve();
    await flushPromises();
    // 加载完成 → 云图热更新为新场
    expect(backend.setFaceValues).toHaveBeenCalled();

    panel.togglePlay();
    expect(panel.playLabel).toBe("播放动画");
    // 停止后不再推进
    const calls = vi.mocked(loadResultField).mock.calls.length;
    await vi.advanceTimersByTimeAsync(1200);
    expect(vi.mocked(loadResultField).mock.calls.length).toBe(calls);
    panel.unmount();
  });

  it("onUnmounted 停止播放", async () => {
    vi.useFakeTimers();
    const { panel } = await mountLoaded();
    const results = useResultsStore();
    const { loadResultField } = (await import("../../../../src-web/api/results")) as unknown as {
      loadResultField: Mock;
    };
    results.resultCatalog = {
      caseDir: "/case",
      times: [{ dirName: "1", timeS: 1, fields: ["p"] }],
    };
    results.loadedField = makeField([1, 2, 3, 4]);
    vi.mocked(loadResultField).mockResolvedValue(makeField([2, 3, 4, 5]));

    panel.togglePlay();
    const { unmount } = { unmount: () => panel.unmount() };
    unmount();
    const calls = vi.mocked(loadResultField).mock.calls.length;
    await vi.advanceTimersByTimeAsync(1200);
    expect(vi.mocked(loadResultField).mock.calls.length).toBe(calls);
  });
});

describe("useViewportPanel：网格载入边界", () => {
  it("无几何时不请求渲染网格，标题为空（未载入分支）", async () => {
    const { panel } = mountPanel();
    const { getRenderMesh } = (await import("../../../../src-web/api/geometry")) as unknown as {
      getRenderMesh: ReturnType<typeof vi.fn>;
    };
    expect(panel.title).toBe("");
    await panel.loadMesh();
    expect(getRenderMesh).not.toHaveBeenCalled();
    panel.unmount();
  });

  it("非主视口创建失败不置空态错误；画布卸载后到达的创建不注册监听", async () => {
    const { panel } = await mountLoaded();
    const viewport = useViewportStore();
    viewport.setLayout("quad");
    await nextTick();

    // 四分格实例创建失败：非主视口不写 emptyError
    createMock.mockImplementationOnce(async () => null);
    panel.attachCanvas(1, canvas());
    await flushPromises();
    expect(panel.emptyError).toBe(false);

    // 主视口画布卸载后，在途创建完成：el 已为 null，指针监听不注册
    const el = document.createElement("canvas");
    panel.attachCanvas(2, el);
    panel.attachCanvas(2, null);
    await flushPromises();
    const stray = new Event("pointerup");
    Object.assign(stray, { clientX: 0, clientY: 0 });
    el.dispatchEvent(stray);
    expect(pickCellMock).not.toHaveBeenCalled();
    panel.unmount();
  });

  it("渲染网格获取失败（store 错误管道返回 undefined）时跳过上传", async () => {
    const geometry = useGeometryStore();
    const app = (await import("../../../../src-web/stores/app")).useAppStore();
    const { getRenderMesh } = (await import("../../../../src-web/api/geometry")) as unknown as {
      getRenderMesh: ReturnType<typeof vi.fn>;
    };
    geometry.geometries = [
      {
        geometryId: "g-1",
        fileName: "demo.stl",
        triangleCount: 2,
        size: [1, 1, 0],
        surfaceArea: 1,
        signedVolume: 0,
        suggestedUnit: "mm",
        issues: { degenerate: 0, openEdges: 0, nonManifoldEdges: 0, normalInconsistentEdges: 0 },
      },
    ];
    vi.mocked(getRenderMesh).mockRejectedValue(new Error("几何不存在"));
    const { panel } = mountPanel();
    panel.attachCanvas(0, canvas());
    await flushPromises();
    await panel.loadMesh();
    await flushPromises();
    expect(app.error?.message).toBe("几何不存在");
    expect(panel.meshReady).toBe(false);
    panel.unmount();
  });

  it("attachCanvas 卸载（el=null）与已有渲染器时幂等", async () => {
    const { panel } = await mountLoaded();
    panel.attachCanvas(1, null);
    // 已有渲染器的主视口再次 attach：不重复创建
    panel.attachCanvas(0, canvas());
    await flushPromises();
    expect(createMock).toHaveBeenCalledTimes(1);
    panel.unmount();
  });
});
