/** 3D 视口面板：多实例视口（单视口 / 四分格）+ 云图 / 剖切 / 时间步动画 /
 * 空间拾取。布局与图层意图存于视口 store；相机在实例间联动——任一实例
 * 交互后，其余实例沿其轨道相机跟随（时间轴经 results store 天然同步）。 */
import { computed, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { useGeometryStore } from "../../stores/geometry";
import { useProjectStore } from "../../stores/project";
import { useResultsStore } from "../../stores/results";
import { useViewportStore, type ViewportLayout } from "../../stores/viewport";
import type { ScalarField } from "../../types";
import { createViewportRenderer, type ViewportBackend } from "../../render/backend";
import { pickCell, rayFromPointer, type PickMesh } from "../../render/picking";
import { buildOverlayLayers, OVERLAY_IDS } from "../../render/overlays";
import { detectRenderCapabilityInBrowser } from "../../render/capability";
import { registerSnapshot } from "../../render/snapshot";
import { clipPlaneFromFraction } from "../../render/math";
import { minMax, quickselect } from "../../utils/stats";
import { createFieldAnimation } from "../../utils/animation";

interface ViewportSlot {
  id: number;
  el: HTMLCanvasElement | null;
  renderer: ViewportBackend | null;
  renderMesh: PickMesh | null;
  /** 该实例是否已载入网格（供模板显示占位）。 */
  loaded: boolean;
}

export function useViewportPanel() {
  const geometry = useGeometryStore();
  const results = useResultsStore();
  const project = useProjectStore();
  const viewport = useViewportStore();

  // —— 多实例布局 ——
  // slot 0 为主视口（FPS 读数 / 快照来源）；四分格时每个实例独立画布与相机，
  // 任一实例交互即把轨道相机复制到其余实例（联动）。
  const layout = computed<ViewportLayout>(() => viewport.layout);
  const slotIds = computed(() => (viewport.layout === "quad" ? [0, 1, 2, 3] : [0]));
  const slots = reactive(
    [0, 1, 2, 3].map((id): ViewportSlot => ({
      id,
      el: null,
      renderer: null,
      renderMesh: null,
      loaded: false,
    })),
  );
  function slotById(id: number): ViewportSlot {
    return slots[id] as ViewportSlot;
  }

  /** 模板 ref 回调：画布挂载 / 卸载时维护实例的元素并按需补建渲染器。 */
  function attachCanvas(id: number, el: unknown): void {
    const slot = slotById(id);
    slot.el = (el as HTMLCanvasElement | null) ?? null;
    // 已有共享网格数据而该实例尚未就绪（如切到四分格后新增实例）→ 立即补建。
    if (slot.el !== null && sharedMesh !== null) {
      void setupSlot(slot);
    }
  }

  // 空态提示：载入网格前主视口不应是一片空白；探测备注动态替换。
  const emptyText = ref(
    "导入几何并生成网格后，点击「载入网格到视口」查看 3D 模型（WebGPU 可用时自动启用）",
  );
  const emptyError = ref(false);
  const meshLoaded = computed(() => slots.some((slot) => slot.loaded));

  // 悬浮色标图例：数值自上而下 max/mid/min，与渐变条方向对应。
  const legendValues = ref<number[] | null>(null);
  const legendVisible = computed(() => legendValues.value !== null);

  function updateLegend(field: { values: number[] } | null): void {
    if (field === null || field.values.length === 0) {
      legendValues.value = null;
      return;
    }
    const values = field.values;
    // min/max 线性扫描；中值 quickselect（O(n) 平均），大场动画逐帧调用
    // 时不做 O(n log n) 全量排序。
    const { min, max } = minMax(values);
    const mid = quickselect([...values], Math.floor(values.length / 2));
    legendValues.value = [max, mid, min];
  }

  // —— 控制条 ——
  const loadDisabled = computed(() => geometry.geometries.length === 0);
  const meshReady = computed(() => meshLoaded.value);
  const playDisabled = computed(() => {
    const catalog = results.resultCatalog;
    return !meshReady.value || catalog === null || catalog.times.length === 0;
  });
  const fpsText = ref("FPS: —");
  const clipOn = ref(false);
  const clipAxis = ref<"x" | "y" | "z">("y");
  const clipPosition = ref(0.5);
  const clipInvert = ref(false);
  const playing = ref(false);
  const playLabel = computed(() => (playing.value ? "停止动画" : "播放动画"));

  /** 共享网格数据：一次获取，所有实例（含后续新增）复用上传。 */
  let sharedMesh: PickMesh | null = null;
  /** 时间步动画节拍器：背压/回绕/停止逻辑在 utils/animation（可单测）。 */
  let animation: ReturnType<typeof createFieldAnimation> | null = null;

  // —— 空间拾取 ——
  // pointerdown/up 位移小于阈值视为点击（大于阈值是旋转拖拽），命中单元加入探针。
  const PICK_SLOP_PIXELS = 4;
  const probeHits = ref(0);

  function attachPointerHandlers(slot: ViewportSlot): void {
    const el = slot.el;
    if (el === null) {
      return;
    }
    let down: { x: number; y: number } | null = null;
    el.addEventListener("pointerdown", (event) => {
      down = { x: event.clientX, y: event.clientY };
    });
    el.addEventListener("pointerup", (event) => {
      const start = down;
      down = null;
      if (start === null || slot.renderer === null || slot.renderMesh === null) {
        return;
      }
      if (Math.hypot(event.clientX - start.x, event.clientY - start.y) > PICK_SLOP_PIXELS) {
        return;
      }
      const rect = el.getBoundingClientRect();
      const ray = rayFromPointer(
        slot.renderer.getCamera(),
        event.clientX - rect.left,
        event.clientY - rect.top,
      );
      const hit = pickCell(slot.renderMesh, ray);
      if (hit !== null) {
        results.addProbe(hit.cell);
        probeHits.value += 1;
      }
    });
  }

  // 相机注视点读数（主视口）：任一实例交互时更新。
  const viewCenter = ref({ x: 0, y: 0, z: 0 });
  const centerText = computed(
    () =>
      `X ${viewCenter.value.x.toFixed(1)} · Y ${viewCenter.value.y.toFixed(1)} · Z ${viewCenter.value.z.toFixed(1)}`,
  );

  /** 相机联动：源实例交互后，把其轨道相机复制到其余实例。 */
  function syncOrbitFrom(source: ViewportSlot): void {
    const orbit = source.renderer?.getOrbit();
    if (orbit === undefined) {
      return;
    }
    viewCenter.value = { x: orbit.x, y: orbit.y, z: orbit.z };
    for (const slot of slots) {
      if (slot.id !== source.id && slot.renderer !== null) {
        slot.renderer.setOrbit(orbit);
      }
    }
  }

  // 视口标题：当前几何与研究（未载入网格时不显示）。
  const title = computed(() => {
    if (!meshLoaded.value) {
      return "";
    }
    const fileName = geometry.geometries[0]?.fileName ?? "—";
    return `${fileName} · ${project.activeStudy?.name ?? "未选择方案"}`;
  });

  function stopPlay(): void {
    playing.value = false;
    animation?.stop();
  }

  function startPlay(): void {
    updateLegend(results.loadedField);
    const catalog = results.resultCatalog;
    if (catalog === null || catalog.times.length === 0) {
      return;
    }
    const caseDir = catalog.caseDir;
    const fieldName = results.loadedField?.field ?? "T";
    playing.value = true;
    animation ??= createFieldAnimation(400);
    animation.start(
      catalog.times.map((step) => step.dirName),
      async (dirName) => {
        await results.loadField(caseDir, dirName, fieldName);
        applyField(results.loadedField);
      },
    );
  }

  function togglePlay(): void {
    if (playing.value) {
      stopPlay();
    } else {
      startPlay();
    }
  }

  function applyClip(): void {
    for (const slot of slots) {
      if (slot.renderer === null || slot.renderMesh === null) {
        continue;
      }
      const bounds = slot.renderer.getMeshBounds();
      if (bounds === null) {
        continue;
      }
      const plane = clipPlaneFromFraction(
        bounds.min,
        bounds.max,
        clipAxis.value,
        clipPosition.value,
        clipInvert.value,
      );
      slot.renderer.setClipPlane(clipOn.value, plane.normal, plane.offset);
    }
  }

  function toggleClip(): void {
    clipOn.value = !clipOn.value;
    applyClip();
  }

  function resetView(): void {
    for (const slot of slots) {
      slot.renderer?.resetView();
    }
    syncOrbitFrom(slots[0]!);
  }

  function zoomBy(factor: number): void {
    // 以主视口为源缩放，联动到其余实例（等比传递避免多次缩放叠加）。
    slots[0]?.renderer?.zoomBy(factor);
    syncOrbitFrom(slots[0]!);
  }

  function fitView(): void {
    for (const slot of slots) {
      slot.renderer?.fitView();
    }
  }

  async function ensureSlotRenderer(slot: ViewportSlot): Promise<void> {
    if (slot.renderer !== null || slot.el === null) {
      return;
    }
    const created = await createViewportRenderer(
      slot.el,
      slot.id === 0
        ? (fps) => {
            fpsText.value = `FPS: ${fps}`;
          }
        : undefined,
      (state) => {
        viewCenter.value = state;
        syncOrbitFrom(slot);
      },
    );
    if (created === null) {
      if (slot.id === 0) {
        emptyText.value = "当前环境不支持 WebGL2 / WebGPU，无法渲染视口。";
        emptyError.value = true;
      }
      return;
    }
    slot.renderer = created.backend;
    attachPointerHandlers(slot);
  }

  /** 单实例就绪：建渲染器 + 上传共享网格 + 应用云图 / 剖切 / 图层。 */
  async function setupSlot(slot: ViewportSlot): Promise<void> {
    if (sharedMesh === null || slot.el === null || slot.loaded) {
      return;
    }
    await ensureSlotRenderer(slot);
    const renderer = slot.renderer;
    if (renderer === null) {
      return;
    }
    renderer.uploadMesh({
      positions: sharedMesh.positions,
      indices: sharedMesh.indices,
      faceCells: sharedMesh.faceCells,
    });
    uploadOverlays(slot);
    applyLayerVisibility();
    applyField(results.loadedField);
    applyClip();
    slot.loaded = true;
  }

  async function loadMesh(): Promise<void> {
    const first = geometry.geometries[0];
    if (first === undefined) {
      return;
    }
    // 渲染网格经 geometry store（错误进全局管道）；无数据（失败）则跳过上传。
    const data = await geometry.fetchRenderMesh(first.geometryId);
    if (data === undefined) {
      return;
    }
    sharedMesh = {
      positions: new Float32Array(data.positions),
      indices: new Uint32Array(data.indices),
      faceCells: new Uint32Array(data.faceCells),
    };
    for (const slot of slots) {
      await setupSlot(slot);
    }
    if (slots[0]?.loaded === true) {
      registerSnapshot("viewport", slots[0]!.el!);
    }
  }

  /** 把当前研究的浇注系统 / 冷却水路上传为线段叠加层（全部实例）。 */
  function uploadOverlays(slot: ViewportSlot): void {
    const study = project.activeStudy;
    if (study === null || slot.renderer === null) {
      return;
    }
    const layers = buildOverlayLayers(study);
    slot.renderer.uploadOverlay(OVERLAY_IDS.gates, layers.gates);
    slot.renderer.uploadOverlay(OVERLAY_IDS.runners, layers.runners);
    slot.renderer.uploadOverlay(OVERLAY_IDS.cooling, layers.cooling);
  }

  /** 把图层可见性意图同步到渲染器（层管理面板与视口的桥）。 */
  function applyLayerVisibility(): void {
    for (const slot of slots) {
      if (slot.renderer === null) {
        continue;
      }
      slot.renderer.setMeshVisible(viewport.layers.mesh);
      slot.renderer.setOverlayVisible(OVERLAY_IDS.gates, viewport.layers.gates);
      slot.renderer.setOverlayVisible(OVERLAY_IDS.runners, viewport.layers.runners);
      slot.renderer.setOverlayVisible(OVERLAY_IDS.cooling, viewport.layers.cooling);
    }
  }

  watch(
    () => viewport.layers,
    () => applyLayerVisibility(),
  );

  watch([clipAxis, clipPosition, clipInvert], () => applyClip());

  // 场数据加载后自动开启云图着色（值域取自场 min/max），并热更新每面值。
  function applyField(field: ScalarField | null): void {
    if (field === null || field.values.length === 0) {
      return;
    }
    for (const slot of slots) {
      if (slot.renderer === null || slot.renderMesh === null) {
        continue;
      }
      const perFace = new Float32Array(slot.renderMesh.faceCells.length);
      for (let face = 0; face < perFace.length; face += 1) {
        perFace[face] = field.values[slot.renderMesh.faceCells[face] ?? 0] ?? 0;
      }
      slot.renderer.setFaceValues(perFace);
      const { min, max } = minMax(field.values);
      slot.renderer.setFieldRange(min, max);
    }
  }

  watch(
    () => results.loadedField,
    (field) => {
      updateLegend(field);
      applyField(field);
    },
  );

  // 布局切换后：四分格新增实例补建（共享网格已缓存），时间轴 / 云图 / 图层自动就绪。
  watch(layout, () => {
    for (const slot of slots) {
      void setupSlot(slot);
    }
  });

  onMounted(() => {
    void detectRenderCapabilityInBrowser().then((capability) => {
      if (capability.backend === "webgl2") {
        emptyText.value = `${emptyText.value}（${capability.note}）`;
      }
    });
  });
  onUnmounted(stopPlay);

  return {
    layout,
    slotIds,
    attachCanvas,
    slots,
    emptyText,
    emptyError,
    meshLoaded,
    probeHits,
    legendVisible,
    legendValues,
    loadDisabled,
    meshReady,
    playDisabled,
    playLabel,
    fpsText,
    clipOn,
    clipAxis,
    clipPosition,
    clipInvert,
    title,
    centerText,
    loadMesh,
    togglePlay,
    toggleClip,
    applyClip,
    resetView,
    zoomBy,
    fitView,
  };
}
